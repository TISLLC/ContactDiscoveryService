FROM ubuntu:bionic

RUN apt-get update && apt-get install -y openjdk-11-jre-headless

COPY linux-sgx.gpg /tmp/docker/
ARG SGX_VER=2.14.100.2-bionic1

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    mc \
    kmod \
    apt-transport-https \
    curl \
    gpg-agent \
    libseccomp2 \
    libseccomp-dev \
    libssl-dev \
    pkg-config \
    protobuf-compiler \
    software-properties-common \
  && apt-key add /tmp/docker/linux-sgx.gpg \
  && apt-add-repository "deb https://download.01.org/intel-sgx/sgx_repo/ubuntu bionic main" \
  && apt-get install -y --download-only \
    libsgx-epid=${SGX_VER} \
    libsgx-quote-ex=${SGX_VER} \
    libsgx-enclave-common=${SGX_VER} \
    libsgx-urts=${SGX_VER} \
    libsgx-ae-le=${SGX_VER} \
    libsgx-launch=${SGX_VER} \
    sgx-aesm-service=${SGX_VER} \
    libsgx-aesm-launch-plugin=${SGX_VER} \
    libsgx-uae-service=${SGX_VER} \
  && dpkg --unpack /var/cache/apt/archives/libsgx-epid_*.deb \
  && dpkg --unpack /var/cache/apt/archives/libsgx-quote-ex_*.deb \
  && dpkg --unpack /var/cache/apt/archives/libsgx-enclave-common_*.deb \
  && dpkg --unpack /var/cache/apt/archives/libsgx-urts_*.deb \
  && dpkg --unpack /var/cache/apt/archives/libsgx-ae-le_*.deb \
  && dpkg --unpack /var/cache/apt/archives/libsgx-launch_*.deb \
  && dpkg --unpack /var/cache/apt/archives/sgx-aesm-service_*.deb \
  && dpkg --unpack /var/cache/apt/archives/libsgx-aesm-launch-plugin_*.deb \
  && dpkg --unpack /var/cache/apt/archives/libsgx-uae-service_*.deb \
  && dpkg --install /var/cache/apt/archives/libsgx-epid_*.deb \
  && dpkg --install /var/cache/apt/archives/libsgx-quote-ex_*.deb \
  && dpkg --install /var/cache/apt/archives/libsgx-enclave-common_*.deb \
  && dpkg --install /var/cache/apt/archives/libsgx-urts_*.deb \
  && dpkg --install /var/cache/apt/archives/libsgx-ae-le_*.deb \
  && dpkg --install /var/cache/apt/archives/libsgx-launch_*.deb \
  && dpkg --install /var/cache/apt/archives/sgx-aesm-service_*.deb \
  && dpkg --install /var/cache/apt/archives/libsgx-aesm-launch-plugin_*.deb \
  && dpkg --install /var/cache/apt/archives/libsgx-uae-service_*.deb \
  && rm -rf /var/lib/apt/lists/*

RUN mkdir /var/run/aesmd && chown aesmd /var/run/aesmd

CMD [ "/bin/bash" ]
