FROM ubuntu:latest

RUN apt-get update && \
    apt-get install -y curl git less gcc

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

RUN git clone https://github.com/dandavison/delta.git
WORKDIR delta
RUN /root/.cargo/bin/cargo build --release

ENV PATH="${PWD}/target/release:${PATH}"

# Add non-root user for security
RUN groupadd -r appuser && useradd -r -g appuser appuser
USER appuser

CMD delta
