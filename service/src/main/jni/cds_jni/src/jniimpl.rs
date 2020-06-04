/*
 * Copyright (C) 2020 Signal Messenger, LLC.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use cds_enclave_ffi::sgxsd;
use jni::objects::*;
use jni::sys::*;
use jni::{sys, Executor, JNIEnv};
use sgx_sdk_ffi::{SgxEnclaveId, SgxStatus};
use std::mem::{size_of, ManuallyDrop};
use std::panic::{catch_unwind, UnwindSafe};
use std::slice;
use std::sync::Arc;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
enum PossibleError {
    // FIXME handle more than just SgxException in server callbacks or maybe not?
    #[error("Exception raised inside CDS JNI, class `{0}`, msg `{1}`", .class, .msg)]
    Generic { class: String, msg: String },
    #[error("SGX call for {0} failed with code {1}", .name, .code)]
    SgxError { name: &'static str, code: i64 },
    #[error("Already thrown Java exception")]
    AlreadyThrown(jni::errors::Error),
}

impl From<jni::errors::Error> for PossibleError {
    fn from(e: jni::errors::Error) -> Self {
        Self::AlreadyThrown(e)
    }
}

impl From<sgxsd::SgxsdError> for PossibleError {
    fn from(e: sgxsd::SgxsdError) -> Self {
        let name = e.name;
        let code = match e.status {
            SgxStatus::Success => panic!("SgxsdError had a successful state"),
            SgxStatus::Error(err) => err as i64,
            SgxStatus::Unknown(code) => code as i64,
        };
        return Self::SgxError {
            name: name,
            code: code,
        };
    }
}

const SGX_EXCEPTION_CLASS: &'static str =
    "org/whispersystems/contactdiscovery/enclave/SgxException";
const SGX_EXCEPTION_CSTOR: &'static str = "(Ljava/lang/String;J)V";

const SGX_NEGOTIATION_RESPONSE_CLASS: &'static str =
    "org/whispersystems/contactdiscovery/enclave/SgxRequestNegotiationResponse";
const SGX_NEGOTIATION_RESPONSE_CSTOR: &'static str = "([B[B[B[B[B)V";

const RUNTIME_EXCEPTION_CLASS: &'static str = "java/lang/RuntimeException";
const NULL_POINTER_EXCEPTION_CLASS: &'static str = "java/lang/NullPointerException";

const NATIVE_CALL_ARGS_CLASS: &'static str =
    "org/whispersystems/contactdiscovery/enclave/SgxEnclave$NativeServerCallArgs";

fn throw_sgx_name_code_to_exception(env: JNIEnv, name: &'static str, code: i64) {
    let _ignore = env
        .new_string(name)
        .and_then(|jstr| {
            let args = &[JValue::Object(jstr.into()), JValue::Long(code)];
            env.new_object(
                SGX_EXCEPTION_CLASS.to_string(),
                SGX_EXCEPTION_CSTOR.to_string(),
                args,
            )
        })
        .map(|exc| env.throw(JThrowable::from(exc)));
}

fn jni_catch<'a, T>(
    env: JNIEnv,
    default: T,
    fun: impl FnOnce() -> Result<T, PossibleError> + UnwindSafe,
) -> T {
    match catch_unwind(fun) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => {
            match error {
                PossibleError::Generic { class, msg } => {
                    let _ignore = env.throw_new(class, msg);
                }
                PossibleError::SgxError { name, code } => {
                    throw_sgx_name_code_to_exception(env, name, code)
                }
                PossibleError::AlreadyThrown(_err) => {
                    // do nothing, it's already been thrown
                }
            }
            default
        }
        Err(_) => {
            let _ignore = env.exception_check().and_then(|jni_has_exc| {
                if !jni_has_exc {
                    return env.throw_new(RUNTIME_EXCEPTION_CLASS, "rust panicked");
                }
                Ok(())
            });
            default
        }
    }
}

fn generic_exception(class: &str, msg: &str) -> PossibleError {
    PossibleError::Generic {
        class: class.to_string(),
        msg: msg.to_string(),
    }
}

#[allow(non_snake_case)]
pub fn Java_org_whispersystems_contactdiscovery_enclave_SgxEnclave_nativeEnclaveStart<'a>(
    env: JNIEnv,
    _class: JClass<'a>,
    enclave_path: JString<'a>,
    debug: jboolean,
    pending_requests_table_order: jbyte,
    callback: JObject<'a>,
) {
    jni_catch(env.clone(), 0, || {
        enclave_start(
            env,
            enclave_path,
            debug == 1,
            pending_requests_table_order,
            callback,
        )
    });
}

fn sgxstatus_to_possibleerror(name: &'static str, status: SgxStatus) -> PossibleError {
    return match status {
        SgxStatus::Success => PossibleError::Generic {
            class: RUNTIME_EXCEPTION_CLASS.to_string(),
            msg: "SgxStatus was a Success but returned as an error in".to_string() + name,
        },
        SgxStatus::Error(err) => PossibleError::SgxError {
            name: name,
            code: err as i64,
        },
        SgxStatus::Unknown(code) => PossibleError::SgxError {
            name: name,
            code: code as i64,
        },
    };
}

fn enclave_start<'a>(
    env: JNIEnv,
    enclave_path: JString,
    debug: bool,
    pending_requests_table_order: i8,
    callback: JObject<'a>,
) -> Result<i64, PossibleError> {
    if callback.is_null() {
        return Err(generic_exception(
            NULL_POINTER_EXCEPTION_CLASS,
            "EnclaveStartCallback callback was null",
        ));
    }
    let enclave_path = env.get_string(enclave_path)?;
    let enclave_path_c = enclave_path.to_str().or(Err(generic_exception(
        RUNTIME_EXCEPTION_CLASS,
        "non-UTF8 bytes in enclave path",
    )))?;

    let (gid, _) = sgx_sdk_ffi::init_quote()
        .map_err(|status| sgxstatus_to_possibleerror("init_quote_before_create", status))?;

    let enclave_id =
        sgxsd::sgxsd_create_enclave(enclave_path_c, debug).map_err(PossibleError::from)?;
    let enclave_id_j = enclave_id as i64;
    sgxsd::sgxsd_node_init(enclave_id, pending_requests_table_order as u8)?;
    env.call_method(
        callback,
        "runEnclave",
        "(JJ)V",
        &[JValue::Long(enclave_id_j), JValue::Long(gid as i64)],
    )?;
    return sgxsd::sgxsd_destroy_enclave(enclave_id)
        .map(|_| enclave_id_j)
        .map_err(PossibleError::from);
}

#[allow(non_snake_case)]
pub fn Java_org_whispersystems_contactdiscovery_enclave_SgxEnclave_nativeGetNextQuote(
    env: JNIEnv,
    class: JClass,
    enclave_id: jlong,
    spid: jbyteArray,
    sig_rl: jbyteArray,
) -> jbyteArray {
    return jni_catch(env.clone(), env.new_byte_array(0).unwrap(), || {
        get_next_quote(env, class, enclave_id, spid, sig_rl)
    });
}

fn get_next_quote(
    env: JNIEnv,
    _class: JClass,
    enclave_id: i64,
    spid: jbyteArray,
    sig_rl: jbyteArray,
) -> Result<jbyteArray, PossibleError> {
    let spid_dyn = jni_array_to_vec(&env, spid)?;
    if spid_dyn.len() != 16 {
        let err = PossibleError::SgxError {
            name: "spid_length_incorrect",
            code: sgx_sdk_ffi::SgxError::InvalidParameter as i64,
        };
        return Err(err);
    }
    let spid_c = &mut [0 as u8; 16];
    spid_c.copy_from_slice(spid_dyn.as_slice());

    let sig_rl_c = jni_array_to_vec(&env, sig_rl)?;

    let quote =
        sgxsd::sgxsd_get_next_quote(enclave_id as SgxEnclaveId, spid_c, sig_rl_c.as_slice())?;
    return slice_to_jni_array(&env, quote.data.as_slice());
}

fn slice_to_jni_array(env: &JNIEnv, data: &[u8]) -> Result<jbyteArray, PossibleError> {
    let out = env.new_byte_array(data.len() as i32)?;
    let buf: Vec<i8> = data.into_iter().map(|i| *i as i8).collect();
    env.set_byte_array_region(out, 0, buf.as_slice())?;
    Ok(out)
}

fn jni_array_to_fixed_buffer(
    env: &JNIEnv,
    jni_array: jbyteArray,
    out: &mut [u8],
) -> Result<(), PossibleError> {
    let length = env.get_array_length(jni_array)?;
    if length as usize != out.len() {
        return Err(PossibleError::Generic {
            class: RUNTIME_EXCEPTION_CLASS.to_string(),
            msg: format!(
                "expected {0} length, got {1} length array",
                out.len(),
                length
            ),
        });
    }
    let outi8 = &mut vec![0 as i8; out.len()];
    env.get_byte_array_region(jni_array, 0, outi8)?;
    for (i, item) in outi8.into_iter().enumerate() {
        out[i] = *item as u8;
    }
    return Ok(());
}

fn jni_array_to_vec(env: &JNIEnv, jni_array: jbyteArray) -> Result<Vec<u8>, PossibleError> {
    let length = env.get_array_length(jni_array)?;
    let outi8 = &mut vec![0; length as usize];
    env.get_byte_array_region(jni_array, 0, outi8)?;
    let out: Vec<u8> = outi8.into_iter().map(|i| *i as u8).collect();
    return Ok(out);
}

#[allow(non_snake_case)]
pub fn Java_org_whispersystems_contactdiscovery_enclave_SgxEnclave_nativeSetCurrentQuote(
    env: JNIEnv,
    _class: JClass,
    enclave_id: jlong,
) {
    jni_catch(env.clone(), (), || set_current_quote(enclave_id))
}

fn set_current_quote(enclave_id: jlong) -> Result<(), PossibleError> {
    return sgxsd::sgxsd_set_current_quote(enclave_id as u64).map_err(PossibleError::from);
}

#[allow(non_snake_case)]
pub fn Java_org_whispersystems_contactdiscovery_enclave_SgxEnclave_nativeNegotiateRequest(
    env: JNIEnv,
    _class: JClass,
    enclave_id: jlong,
    client_pubkey: jbyteArray,
) -> jni::sys::jobject {
    return jni_catch(env.clone(), JObject::null().into_inner(), || {
        negotiate_request(&env, enclave_id, client_pubkey)
    });
}

fn negotiate_request(
    env: &JNIEnv,
    enclave_id: i64,
    client_pubkey: jbyteArray,
) -> Result<sys::jobject, PossibleError> {
    let pubkey = jni_array_to_vec(&env, client_pubkey)?;
    if pubkey.len() as u32 != sgxsd::SGXSD_CURVE25519_KEY_SIZE {
        return Err(PossibleError::SgxError {
            name: "negotiate_request_client_pubkey_copy",
            code: sgx_sdk_ffi::SgxError::InvalidParameter as i64,
        });
    }
    let pubkey_c = &mut [0 as u8; 32];
    pubkey_c.copy_from_slice(pubkey.as_slice());
    let request = sgxsd::SgxsdRequestNegotiationRequest {
        client_pubkey: sgxsd::SgxsdCurve25519PublicKey { x: *pubkey_c },
    };
    let resp = sgxsd::sgxsd_negotiate_request(enclave_id as u64, &request)?;
    let server_static = slice_to_jni_array(&env, &resp.server_static_pubkey.x[..])?;
    let server_ephemeral = slice_to_jni_array(&env, &resp.server_ephemeral_pubkey.x[..])?;
    let data = slice_to_jni_array(&env, &resp.encrypted_pending_request_id.data[..])?;
    let iv = slice_to_jni_array(&env, &resp.encrypted_pending_request_id.iv.data[..])?;
    let mac = slice_to_jni_array(&env, &resp.encrypted_pending_request_id.mac.data[..])?;
    let args = &[
        JValue::from(server_static),
        JValue::from(server_ephemeral),
        JValue::from(data),
        JValue::from(iv),
        JValue::from(mac),
    ];
    return env
        .new_object(
            SGX_NEGOTIATION_RESPONSE_CLASS,
            SGX_NEGOTIATION_RESPONSE_CSTOR,
            args,
        )
        .map(|o| o.into_inner())
        .map_err(PossibleError::from);
}

#[allow(non_snake_case)]
pub fn Java_org_whispersystems_contactdiscovery_enclave_SgxEnclave_nativeServerStart(
    env: JNIEnv,
    _class: JClass,
    enclave_id: jlong,
    state_handle: jlong,
    max_query_phones: jint,
) {
    return jni_catch(env.clone(), (), || {
        server_start(env, enclave_id, state_handle, max_query_phones)
    });
}

fn server_start(
    _env: JNIEnv,
    enclave_id: i64,
    state_handle: i64,
    max_query_phones: i32,
) -> Result<(), PossibleError> {
    let args = sgxsd::SgxsdServerInitArgs {
        max_query_phones: max_query_phones as u32,
        max_ratelimit_states: 0,
    };
    return sgxsd::sgxsd_server_start(enclave_id as u64, &args, state_handle as u64)
        .map_err(PossibleError::from);
}

#[allow(non_snake_case)]
pub fn Java_org_whispersystems_contactdiscovery_enclave_SgxEnclave_nativeServerCall(
    env: JNIEnv,
    _class: JClass,
    enclave_id: jlong,
    state_handle: jlong,
    args: JObject,
    callback: JObject,
) {
    return jni_catch(env.clone(), (), || {
        server_call(env, enclave_id, state_handle, args, callback)
    });
}

fn server_call(
    env: JNIEnv,
    enclave_id: i64,
    state_handle: i64,
    args: JObject,
    callback: JObject,
) -> Result<(), PossibleError> {
    let is_instance = env.is_instance_of(args, NATIVE_CALL_ARGS_CLASS)?;
    if !is_instance {
        return Err(generic_exception(
            RUNTIME_EXCEPTION_CLASS,
            "server_call called with an incorrect callback arguments type",
        ));
    }
    let msg_data = jni_array_to_vec(&env, get_nonnull_byte_array_field(&env, args, "msg_data")?)?;
    if msg_data.len() == 0 {
        return Err(PossibleError::SgxError {
            name: "bad_msg_data",
            code: 0,
        });
    }
    let mut query_data = jni_array_to_vec(
        &env,
        get_nonnull_byte_array_field(&env, args, "query_data")?,
    )?;
    if query_data.len() == 0 {
        return Err(PossibleError::SgxError {
            name: "bad_query_data",
            code: 0,
        });
    }
    let msg_iv = &mut [0 as u8; size_of::<sgxsd::SgxsdAesGcmIv>()];
    get_nonnull_fixed_size_array_field(&env, args, "msg_iv", &mut msg_iv[..])?;
    let msg_mac = &mut [0 as u8; size_of::<sgxsd::SgxsdAesGcmMac>()];
    get_nonnull_fixed_size_array_field(&env, args, "msg_mac", &mut msg_mac[..])?;

    let query_phone_count = env.get_field(args, "query_phone_count", "I")?.i()?;

    let query_iv = &mut [0 as u8; size_of::<sgxsd::SgxsdAesGcmIv>()];
    get_nonnull_fixed_size_array_field(&env, args, "query_iv", &mut query_iv[..])?;
    let query_mac = &mut [0 as u8; size_of::<sgxsd::SgxsdAesGcmMac>()];
    get_nonnull_fixed_size_array_field(&env, args, "query_mac", &mut query_mac[..])?;
    let query_commitment = &mut [0 as u8; sgxsd::SGXSD_SHA256_HASH_SIZE as usize];
    get_nonnull_fixed_size_array_field(&env, args, "query_commitment", &mut query_commitment[..])?;

    // This one is weird, because the previous C code filled the sgxsd_pending_request_id_t struct directly with GetByteArrayRegion.
    // We'd like to undo the choice to serialize pending_request_id_t to this byte array, but have left
    // to avoid making a change to SgxEnclave$NativeServerCallbackArgs.
    let pending_request_id_bytes = &mut [0 as u8; size_of::<sgxsd::SgxsdPendingRequestId>()];
    get_nonnull_fixed_size_array_field(
        &env,
        args,
        "pending_request_id",
        &mut pending_request_id_bytes[..],
    )?;

    let pending_request_id_data = &mut [0 as u8; size_of::<u64>()];
    pending_request_id_data.clone_from_slice(&pending_request_id_bytes[0..size_of::<u64>() - 1]);

    let u64_and_iv = size_of::<u64>() + size_of::<sgxsd::SgxsdAesGcmIv>();
    let pending_request_id_iv = &mut [0 as u8; size_of::<sgxsd::SgxsdAesGcmIv>()];
    pending_request_id_iv
        .clone_from_slice(&pending_request_id_bytes[size_of::<u64>()..u64_and_iv - 1]);

    let u64_iv_and_mac = u64_and_iv + size_of::<sgxsd::SgxsdAesGcmMac>();
    let pending_request_id_mac = &mut [0 as u8; size_of::<sgxsd::SgxsdAesGcmMac>()];
    pending_request_id_mac
        .clone_from_slice(&pending_request_id_bytes[u64_and_iv..u64_iv_and_mac - 1]);

    let sgxcallargs = sgxsd::SgxsdServerCallArgs {
        query_phone_count: query_phone_count as u32,
        ratelimit_state_size: Default::default(),
        ratelimit_state_uuid: Default::default(),
        ratelimit_state_data: std::ptr::null_mut(),
        query: sgxsd::CDSEncryptedMsg {
            iv: sgxsd::SgxsdAesGcmIv { data: *query_iv },
            mac: sgxsd::SgxsdAesGcmMac { data: *query_mac },
            size: query_data.len() as u32,
            data: query_data.as_mut_ptr(),
        },
        query_commitment: *query_commitment,
    };
    let msg_header = sgxsd::SgxsdMessageHeader {
        iv: sgxsd::SgxsdAesGcmIv { data: *msg_iv },
        mac: sgxsd::SgxsdAesGcmMac { data: *msg_mac },
        pending_request_id: sgxsd::SgxsdPendingRequestId {
            data: *pending_request_id_data,
            iv: sgxsd::SgxsdAesGcmIv {
                data: *pending_request_id_iv,
            },
            mac: sgxsd::SgxsdAesGcmMac {
                data: *pending_request_id_mac,
            },
        },
    };

    let callback_ref = env.new_global_ref(callback)?;
    let jvm = env.get_java_vm()?;
    let exec = Executor::new(Arc::new(jvm));
    // This won't be run on the same thread necessarily, so we have to do the Executor work.
    // Plus, we can't use the return values it has.
    let reply_fun = move |res: sgxsd::SgxsdResult<sgxsd::MessageReply>| -> () {
        let _ignore = res
            .map(|reply| {
                let _ignore = exec
                    .with_attached(|local_env| {
                        jni_catch(local_env.clone(), (), || {
                            run_success_callback(&local_env, reply, callback_ref)
                        });
                        return Ok(());
                    })
                    .map_err(|e| {
                        eprintln!(
                            "unable to run server call callback because of a jni crate error: {:?}",
                            e
                        )
                    });
            })
            .map_err(|e| {
                eprintln!(
                    "got a error from the cds enclave in the server call callback: {:?}",
                    e
                )
            });
    };

    return sgxsd::sgxsd_server_call(
        enclave_id as u64,
        sgxcallargs,
        &msg_header,
        msg_data.as_slice(),
        reply_fun,
        state_handle as u64,
    )
    .map_err(PossibleError::from);
}
fn run_success_callback(
    env: &JNIEnv,
    reply: sgxsd::MessageReply,
    callback_ref: GlobalRef,
) -> Result<(), PossibleError> {
    let data = slice_to_jni_array(&env, &reply.data)?;
    let iv = slice_to_jni_array(&env, &reply.iv.data)?;
    let mac = slice_to_jni_array(&env, &reply.mac.data)?;
    let args: Vec<JValue> = vec![data, iv, mac].into_iter().map(JValue::from).collect();
    return env
        .call_method(
            callback_ref.as_obj(),
            "receiveServerReply",
            "([B[B[B)V",
            &args,
        )
        .map(|_| ())
        .map_err(PossibleError::from);
}
fn get_nonnull_fixed_size_array_field(
    env: &JNIEnv,
    obj: JObject,
    field_name: &str,
    buf: &mut [u8],
) -> Result<(), PossibleError> {
    let array = get_nonnull_byte_array_field(&env, obj, field_name)?;
    return jni_array_to_fixed_buffer(&env, array, buf);
}

fn get_nonnull_byte_array_field(
    env: &JNIEnv,
    obj: JObject,
    field_name: &str,
) -> Result<jbyteArray, PossibleError> {
    let field_obj = env.get_field(obj, field_name, "[B")?.l()?;
    if field_obj.is_null() {
        return Err(PossibleError::Generic {
            class: NULL_POINTER_EXCEPTION_CLASS.to_string(),
            msg: field_name.to_string() + " was null",
        });
    }

    return Ok(field_obj.into_inner() as jbyteArray);
}

#[allow(non_snake_case)]
pub fn Java_org_whispersystems_contactdiscovery_enclave_SgxEnclave_nativeServerStop(
    env: JNIEnv,
    _class: JClass,
    enclave_id: jlong,
    state_handle: jlong,
    in_phones_buf: JObject,
    in_uuids_buf: JObject,
    in_phone_count: jlong,
) {
    return jni_catch(env.clone(), (), || {
        server_stop(
            env,
            enclave_id,
            state_handle,
            in_phones_buf,
            in_uuids_buf,
            in_phone_count,
        )
    });
}

fn server_stop(
    env: JNIEnv,
    enclave_id: i64,
    state_handle: i64,
    in_phones_buf: JObject,
    in_uuids_buf: JObject,
    in_phone_count: i64,
) -> Result<(), PossibleError> {
    let (in_phones_bytes, in_phones_capacity) =
        get_direct_byte_buffer_info(&env, in_phones_buf.into())?;
    let (in_uuids_bytes, in_uuids_capacity) =
        get_direct_byte_buffer_info(&env, in_uuids_buf.into())?;
    if in_phone_count < in_phones_capacity / size_of::<sgxsd::Phone>() as i64 {
        let err = PossibleError::SgxError {
            name: "phone_number_buffer_too_small",
            code: 0,
        };
        return Err(err);
    }
    if in_phone_count < in_uuids_capacity / size_of::<sgxsd::Uuid>() as i64 {
        let err = PossibleError::SgxError {
            name: "uuid_buffer_too_small",
            code: 0,
        };
        return Err(err);
    }
    if in_phone_count < 0 {
        let err = PossibleError::SgxError {
            name: "in_phone_count_too_small",
            code: 0,
        };
        return Err(err);
    }

    let mut in_phones_bytes_undrop = ManuallyDrop::new(in_phones_bytes);
    let mut in_uuids_bytes_undrop = ManuallyDrop::new(in_uuids_bytes);

    // We are abusing the memory layout here because this code is about to be promptly deleted when
    // the DirectoryHashSet code is pushed from Java into Rust. Delete these unsafe calls when that
    // occurs.
    let in_phones = unsafe {
        slice::from_raw_parts_mut(
            in_phones_bytes_undrop.as_mut_ptr() as *mut u64,
            in_phones_bytes_undrop.len() / size_of::<u64>(),
        )
    };
    let in_uuids = unsafe {
        slice::from_raw_parts_mut(
            in_uuids_bytes_undrop.as_mut_ptr() as *mut sgxsd::Uuid,
            in_uuids_bytes_undrop.len() / size_of::<sgxsd::Uuid>(),
        )
    };
    let args = sgxsd::ServerStopArgs {
        in_phones: &mut in_phones[0],
        in_uuids: &mut in_uuids[0],
        in_phone_count: in_phone_count as u64,
    };
    let res = sgxsd::sgxsd_server_stop(enclave_id as u64, &args, state_handle as u64)
        .map_err(PossibleError::from);
    unsafe {
        ManuallyDrop::drop(&mut in_phones_bytes_undrop);
        ManuallyDrop::drop(&mut in_uuids_bytes_undrop);
    }
    return res;
}

fn get_direct_byte_buffer_info<'a>(
    env: &'a JNIEnv,
    buf: JByteBuffer,
) -> Result<(&'a mut [u8], i64), PossibleError> {
    if buf.is_null() {
        return Ok((&mut [], 0));
    }
    return Ok((
        env.get_direct_buffer_address(buf)?,
        env.get_direct_buffer_capacity(buf)?,
    ));
}

#[allow(non_snake_case)]
pub fn Java_org_whispersystems_contactdiscovery_enclave_SgxEnclave_nativeReportPlatformAttestationStatus(
    env: JNIEnv,
    _class: JClass,
    platform_info: jbyteArray,
    attestation_successful: jboolean,
) -> jint {
    return jni_catch(env.clone(), 0, || {
        report_platform_attestation_status(env, platform_info, attestation_successful != 0)
    });
}

fn report_platform_attestation_status(
    env: JNIEnv,
    platform_info_bytes: jbyteArray,
    attestation_successful: bool,
) -> Result<i32, PossibleError> {
    if platform_info_bytes.is_null() {
        return Err(generic_exception(
            NULL_POINTER_EXCEPTION_CLASS,
            "platform_info array cannot be null",
        ));
    }
    let out = &mut [0 as u8; size_of::<sgxsd::SgxPlatformInfo>()];
    jni_array_to_fixed_buffer(&env, platform_info_bytes, out)?;
    let info = sgxsd::SgxPlatformInfo {
        platform_info: *out,
    };
    return sgxsd::sgxsd_report_attestation_status(&info, attestation_successful)
        .map(|attest_status| {
            match attest_status {
                sgxsd::AttestationStatus::NoUpdateNeeded => 0,
                sgxsd::AttestationStatus::UpdateNeeded(update_info) => {
                    // This matches the SgxNeedsUpdateFlag Java enum
                    let ucode = if update_info.ucodeUpdate != 0 { 1 } else { 0 };
                    let csmefw = if update_info.csmeFwUpdate != 0 { 1 } else { 0 };
                    let psw = if update_info.pswUpdate != 0 { 1 } else { 0 };
                    return ucode | csmefw << 1 | psw << 2;
                }
            }
        })
        .map_err(PossibleError::from);
}
