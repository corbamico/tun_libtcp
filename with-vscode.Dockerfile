FROM rust:latest

RUN apt-get update && apt-get install apt-transport-https -y
RUN curl https://packages.microsoft.com/keys/microsoft.asc | gpg --dearmor > microsoft.gpg ; \
    install -o root -g root -m 644 microsoft.gpg /etc/apt/trusted.gpg.d/  ; \
    echo "deb [arch=amd64] https://packages.microsoft.com/repos/vscode stable main" > /etc/apt/sources.list.d/vscode.list ; 
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y \
    tshark \
    netcat \
    libasound2 \
    fish
RUN apt-get update && apt-get install -y code 

RUN rustup component add --toolchain $RUST_VERSION rust-src rls rust-analysis clippy
RUN code --user-data-dir /var/run/code --force \
    --install-extension rust-lang.rust  \
    --install-extension pkief.material-icon-theme \
    --install-extension polypus74.trusty-rusty-snippets

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
