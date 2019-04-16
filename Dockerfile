FROM rust:latest
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y \
    tshark \
    netcat

#cache packages
WORKDIR /home/rust/demo
RUN USER=root cargo init
RUN echo "futures = \"0.1.26\"" >> Cargo.toml
RUN echo "tun = { version = \"0.4.3\" , features = [\"mio\"] }" >> Cargo.toml
RUN echo "tokio=\"0.1.18\"" >> Cargo.toml
RUN echo "libc = \"0.2.51\"" >> Cargo.toml
RUN echo "etherparse = \"0.8.0\"" >> Cargo.toml
RUN echo "bytes = \"0.4.12\"" >> Cargo.toml
RUN cargo check
RUN cd .. && rm -rf demo
