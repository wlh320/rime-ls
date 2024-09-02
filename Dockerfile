FROM alpine:edge as builder

# 1. build rime plugins from source
## 1.1 install dependencies
RUN apk add --no-cache boost-dev capnproto-dev chrpath cmake \ 
glog-dev leveldb-dev libmarisa-dev opencc-dev \ 
samurai yaml-cpp-dev git bash clang gtest-dev

## 1.1 clone git repo
RUN git clone https://github.com/rime/librime --depth=1
WORKDIR /librime
RUN bash install-plugins.sh \
  rime/librime-charcode \
  hchunhui/librime-lua \
  lotem/librime-octagram \
  rime/librime-predict

## 1.2 get lua source code
WORKDIR /librime/plugins/lua
RUN bash action-install.sh

## 1.3 build librime
WORKDIR /librime
RUN cmake -B build -G Ninja \
        -DCMAKE_BUILD_TYPE:STRING=Release \
        -DCMAKE_INSTALL_PREFIX=/usr \
        -DCMAKE_BUILD_WITH_INSTALL_RPATH=ON \
        -DBOOST_USE_CXX11=ON \
        -DBUILD_DATA=ON \
        -DBUILD_MERGED_PLUGINS=OFF \
        -DBUILD_TEST=ON \
        -DENABLE_EXTERNAL_PLUGINS=ON
RUN cmake --build build

# 2. build rime-ls
RUN apk add --no-cache rust cargo musl-dev clang16-libclang librime-dev
WORKDIR /src
COPY . .
RUN cargo build --release

# 3. build rime-ls image
FROM alpine:edge
RUN apk add --no-cache librime
WORKDIR /app
COPY --from=builder /src/target/release/rime_ls /app
COPY --from=builder /librime/build/lib/rime-plugins/ /usr/lib/rime-plugins/
EXPOSE 9257
ENTRYPOINT ["./rime_ls"]
