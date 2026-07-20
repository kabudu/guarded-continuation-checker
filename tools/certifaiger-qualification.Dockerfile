ARG QUALIFICATION_BASE=gcc-ubuntu-24.04-base:v1-arm64
FROM ${QUALIFICATION_BASE}

ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        build-essential=12.10ubuntu1 \
        clang=1:18.0-59~exp2 \
        cmake=3.28.3-1build7 \
        git=1:2.43.0-1ubuntu7.3 \
        libboost-dev=1.83.0.1ubuntu2 \
        libboost-iostreams-dev=1.83.0.1ubuntu2 \
        lld=1:18.0-59~exp2 \
    && rm -rf /var/lib/apt/lists/*
