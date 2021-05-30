ARG UBUNTU_VERSION=18.04
FROM ubuntu:${UBUNTU_VERSION}

ARG UBUNTU_VERSION=18.04
ARG COLLECTD_VERSION=5.7

# So tzdata doesn't prompt
ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    collectd \
    collectd-dev \
    ca-certificates \
    wget \
    curl \
    build-essential \
 && rm -rf /var/lib/apt/lists/*
RUN apt-get update && apt-get install -y llvm-dev libclang-dev clang && rm -rf /var/lib/apt/lists/*
RUN curl https://sh.rustup.rs -sSf | sh -s -- --profile=minimal -y
COPY . /tmp
