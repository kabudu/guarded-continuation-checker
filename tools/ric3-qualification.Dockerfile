ARG QUALIFICATION_BASE=gcc-rust-1.97-bookworm:v1-arm64
FROM ${QUALIFICATION_BASE}

ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        clang=1:14.0-55.7~deb12u1 \
        cmake=3.25.1-1 \
        libmpfr-dev=4.2.0-1 \
        meson=1.0.1-5 \
    && rm -rf /var/lib/apt/lists/*
