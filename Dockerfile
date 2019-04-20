FROM rust:latest
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y \
    tshark \
    netcat

#cache packages
WORKDIR /home/rust/demo
RUN USER=root cargo init ; \
    echo "\
    futures = \"0.1.26\"\n\
    tun = { version = \"0.4.3\" , features = [\"mio\"] }\n\
    tokio=\"0.1.18\"\n\
    libc = \"0.2.51\"\n\
    etherparse = \"0.8.0\"\n\
    bytes = \"0.4.12\"  " >> Cargo.toml; \
    cargo check; \
    cd .. ; \
    rm -rf demo
