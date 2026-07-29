FROM python:3.13-slim@sha256:6771159cd4fa5d9bba1258caf0b82e6b73458c694d178ad97c5e925c2d0e1a91

RUN python -m pip install \
      --disable-pip-version-check \
      --no-cache-dir \
      angr==9.3.0 \
    && python -c "import angr; assert angr.__version__ == '9.3.0'"
