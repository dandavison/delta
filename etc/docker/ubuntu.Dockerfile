FROM ubuntu:20.04

RUN apt-get update && \
    apt-get install -y curl git less

RUN curl -OL https://github.com/dandavison/delta/releases/download/0.4.5/delta-0.4.5-x86_64-unknown-linux-gnu.tar.gz && \
    tar -xzvf delta-0.4.5-x86_64-unknown-linux-gnu.tar.gz

WORKDIR delta-0.4.5-x86_64-unknown-linux-gnu

ENV PATH="${PWD}:${PATH}"

CMD delta
