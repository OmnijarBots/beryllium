FROM ekidd/rust-musl-builder:nightly

RUN sudo apt-get update && \
    sudo apt-get install -y unzip pkg-config && \
    sudo apt-get clean && sudo rm -rf /var/lib/apt/lists/*

RUN VERSION=1.0.15 && \
    cd /home/rust/libs && \
    mkdir libsodium && \
    cd libsodium && \
    curl -L https://download.libsodium.org/libsodium/releases/libsodium-$VERSION.tar.gz -o libsodium-$VERSION.tar.gz && \
    tar xfvz libsodium-$VERSION.tar.gz && \
    cd libsodium-$VERSION/ && \
    ./configure --enable-shared=no && \
    make && make check && \
    sudo make install && \
    sudo mv src/libsodium /usr/local/ && \
    rm -rf /home/rust/libs/libsodium

RUN VERSION=3.4.0 && \
    cd /home/rust/libs && \
    mkdir protoc && \
    cd protoc && \
    curl -L https://github.com/google/protobuf/releases/download/v$VERSION/protoc-$VERSION-linux-x86_64.zip -o protoc-$VERSION.zip && \
    unzip protoc-$VERSION.zip && \
    sudo mv bin/* /usr/local/bin/ && \
    sudo mv include/* /usr/local/include/ && \
    rm -rf /home/rust/libs/protoc

ENV PKG_CONFIG_ALLOW_CROSS 1
ENV SODIUM_LIB_DIR /usr/local/lib
