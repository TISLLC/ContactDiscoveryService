/* DO NOT EDIT THIS FILE - it is machine generated */
#include <jni.h>
/* Header for class org_whispersystems_contactdiscovery_enclave_SgxEnclave */

#ifndef _Included_org_whispersystems_contactdiscovery_enclave_SgxEnclave
#define _Included_org_whispersystems_contactdiscovery_enclave_SgxEnclave
#ifdef __cplusplus
extern "C" {
#endif
#undef org_whispersystems_contactdiscovery_enclave_SgxEnclave_PENDING_REQUESTS_TABLE_ORDER
#define org_whispersystems_contactdiscovery_enclave_SgxEnclave_PENDING_REQUESTS_TABLE_ORDER 16L
/*
 * Class:     org_whispersystems_contactdiscovery_enclave_SgxEnclave
 * Method:    nativeEnclaveStart
 * Signature: (Ljava/lang/String;ZBLorg/whispersystems/contactdiscovery/enclave/SgxEnclave/EnclaveStartCallback;)V
 */
JNIEXPORT void JNICALL Java_org_whispersystems_contactdiscovery_enclave_SgxEnclave_nativeEnclaveStart
  (JNIEnv *, jclass, jstring, jboolean, jbyte, jobject);

/*
 * Class:     org_whispersystems_contactdiscovery_enclave_SgxEnclave
 * Method:    nativeGetNextQuote
 * Signature: (J[B[B)[B
 */
JNIEXPORT jbyteArray JNICALL Java_org_whispersystems_contactdiscovery_enclave_SgxEnclave_nativeGetNextQuote
  (JNIEnv *, jclass, jlong, jbyteArray, jbyteArray);

/*
 * Class:     org_whispersystems_contactdiscovery_enclave_SgxEnclave
 * Method:    nativeSetCurrentQuote
 * Signature: (J)V
 */
JNIEXPORT void JNICALL Java_org_whispersystems_contactdiscovery_enclave_SgxEnclave_nativeSetCurrentQuote
  (JNIEnv *, jclass, jlong);

/*
 * Class:     org_whispersystems_contactdiscovery_enclave_SgxEnclave
 * Method:    nativeNegotiateRequest
 * Signature: (J[B)Lorg/whispersystems/contactdiscovery/enclave/SgxRequestNegotiationResponse;
 */
JNIEXPORT jobject JNICALL Java_org_whispersystems_contactdiscovery_enclave_SgxEnclave_nativeNegotiateRequest
  (JNIEnv *, jclass, jlong, jbyteArray);

/*
 * Class:     org_whispersystems_contactdiscovery_enclave_SgxEnclave
 * Method:    nativeServerStart
 * Signature: (JJI)V
 */
JNIEXPORT void JNICALL Java_org_whispersystems_contactdiscovery_enclave_SgxEnclave_nativeServerStart
  (JNIEnv *, jclass, jlong, jlong, jint);

/*
 * Class:     org_whispersystems_contactdiscovery_enclave_SgxEnclave
 * Method:    nativeServerCall
 * Signature: (JJLorg/whispersystems/contactdiscovery/enclave/SgxEnclave/NativeServerCallArgs;Lorg/whispersystems/contactdiscovery/enclave/SgxEnclave/NativeServerReplyCallback;)V
 */
JNIEXPORT void JNICALL Java_org_whispersystems_contactdiscovery_enclave_SgxEnclave_nativeServerCall
  (JNIEnv *, jclass, jlong, jlong, jobject, jobject);

/*
 * Class:     org_whispersystems_contactdiscovery_enclave_SgxEnclave
 * Method:    nativeServerStop
 * Signature: (JJLjava/nio/ByteBuffer;Ljava/nio/ByteBuffer;J)V
 */
JNIEXPORT void JNICALL Java_org_whispersystems_contactdiscovery_enclave_SgxEnclave_nativeServerStop
  (JNIEnv *, jclass, jlong, jlong, jobject, jobject, jlong);

/*
 * Class:     org_whispersystems_contactdiscovery_enclave_SgxEnclave
 * Method:    nativeReportPlatformAttestationStatus
 * Signature: ([BZ)I
 */
JNIEXPORT jint JNICALL Java_org_whispersystems_contactdiscovery_enclave_SgxEnclave_nativeReportPlatformAttestationStatus
  (JNIEnv *, jclass, jbyteArray, jboolean);

#ifdef __cplusplus
}
#endif
#endif
