ARG UBUNTU_VERSION=16.04
FROM ubuntu:${UBUNTU_VERSION}

# Annoying that UBUNTU_VERSION has to be specified again
ARG UBUNTU_VERSION=16.04
ARG COLLECTD_VERSION=5.5

RUN apt-get update && apt-get install -y --no-install-recommends \
    collectd \
    collectd-dev \
    ca-certificates \
    wget \
    curl \
    build-essential \
 && rm -rf /var/lib/apt/lists/*
RUN if [ "${COLLECTD_VERSION}" != "5.7" ]; then wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key | apt-key add -; fi
RUN apt-get update && apt-get install -y llvm-3.9-dev libclang-3.9-dev clang-3.9 && rm -rf /var/lib/apt/lists/*
RUN if [ "${COLLECTD_VERSION}" = "5.4" ]; then cp -r /usr/include/collectd/liboconfig /usr/include/collectd/core/.; fi
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
COPY . /tmp
