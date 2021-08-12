FROM ubuntu:latest

RUN apt-get update && \
    apt-get install -y curl git less

RUN git clone https://github.com/dandavison/delta.git

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

RUN curl -OL https://github.com/dandavison/delta/releases/download/0.8.3/delta-0.8.3-x86_64-unknown-linux-musl.tar.gz && \
    tar -xzvf delta-0.8.3-x86_64-unknown-linux-musl.tar.gz

WORKDIR delta-0.8.3-x86_64-unknown-linux-musl

ENV PATH="${PWD}:${PATH}"

CMD delta
