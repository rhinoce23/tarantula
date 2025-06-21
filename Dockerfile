FROM rust:alpine3.20

RUN apk add --no-cache bash
RUN apk add --no-cache git
RUN apk add --no-cache jq
RUN apk add --no-cache tzdata
RUN apk add --no-cache make g++ cmake openssl-dev gtest-dev proj-dev swig s2geometry-dev
RUN apk add --no-cache geos-dev
RUN apk add --no-cache postgresql
RUN apk add --no-cache postgis
RUN apk add --no-cache aws-cli

WORKDIR /tarantula
