# Readme

Demostrate rust example tun operation and tcp handshake on tun channel by studying following stream.  

code based on following lecture:  
youtube <https://www.youtube.com/watch?v=bzja9fQWzdA>  
code <https://github.com/jonhoo/rust-tcp>  

## run in docker

### Tips&Notes

>`docker build -t corbamico/rust:1.34 .`  
>`docker run -v /c/project/rust/docker:/home/rust -w /home/rust  --cap-add=NET_ADMIN  --device=/dev/net/tun  -it  corbamico/rust:1.34   /bin/bash`  
>`docker exec -it 678f26b3a034 -u 0:0 /bin/bash`

### run vscode inside docker  

for windows 10, install VcXsrv

>`docker run --rm -v /c/project/rust/docker:/home/rust -w /home/rust/tun_libtcp  --privileged --cap-add=NET_ADMIN  --device=/dev/net/tun  --env DISPLAY=10.0.75.1:0.0 corbamico/rust:vscode code --wait --diable-updates --user-data-dir=/var/run/code .`

* --privileged  
  vscode need dbus  

* --wait
  vscode need xclient link to x-server

* --user-data-dir
  see refence <https://github.com/Microsoft/vscode/issues/28126>

### tips for attach shell from cmd.exe

create fish.bat file as:

>`@pwsh -c "& {$cid=docker ps -q ; docker exec -it $cid.trim() /usr/bin/fish}"`

## Steps

### Step1

* Description

    Print tun data bytes and headers

* how to run

    >`cargo run --bin libtcp_step1 &`  
    >`ping -I tun0 10.0.0.5`

### Step2

* Description

    implement ICMP echo (ping).

* example ping packet

    ```shell
    packet :[69, 0, 0, 36, 3, 119, 64, 0, 64, 1, 35, 93, 10, 0, 0, 1, 10, 0, 0, 5, 8, 0, 232, 126, 3, 112, 0, 1, 0, 1, 2, 3, 4, 5, 6, 7]
    ```

* how to run

    >`cargo run --bin libtcp_step2 &`  
    >`ping -I tun0 10.0.0.100`

### Step3

* Description

    implement TCP server syn/fin packet, and tcp server feedback fix "hello\n"

* how to run

    open 3 shells run following separately
    >`cargo run --bin libtcp_step3`  
    >`tshark -i tun0`  
    >`nc -s 10.0.0.1 10.0.0.5 8000`
* tshark capture as

    ```shell
    1 0.000000000     10.0.0.1 ? 10.0.0.5     TCP 60 34255 ? 8000 [SYN] Seq=0 Win=29200 Len=0 MSS=1460 SACK_PERM=1 TSval=4953968 TSecr=0 WS=128
    2 0.000076200     10.0.0.5 ? 10.0.0.1     TCP 40 8000 ? 34255 [SYN, ACK] Seq=0 Ack=1 Win=1024 Len=0
    3 0.000120800     10.0.0.1 ? 10.0.0.5     TCP 40 34255 ? 8000 [ACK] Seq=1 Ack=1 Win=29200 Len=0
    4 4.393103500     10.0.0.1 ? 10.0.0.5     TCP 47 34255 ? 8000 [PSH, ACK] Seq=1 Ack=1 Win=29200 Len=7
    5 4.393153200     10.0.0.5 ? 10.0.0.1     TCP 42 8000 ? 34255 [ACK] Seq=1 Ack=8 Win=1024 Len=2
    6 4.393166700     10.0.0.1 ? 10.0.0.5     TCP 40 34255 ? 8000 [ACK] Seq=8 Ack=3 Win=29200 Len=0
    7 7.228395400     10.0.0.1 ? 10.0.0.5     TCP 40 34255 ? 8000 [FIN, ACK] Seq=8 Ack=3 Win=29200 Len=0
    8 7.228515200     10.0.0.5 ? 10.0.0.1     TCP 40 8000 ? 34255 [FIN, ACK] Seq=3 Ack=9 Win=1024 Len=0
    9 7.228565900     10.0.0.1 ? 10.0.0.5     TCP 40 34255 ? 8000 [ACK] Seq=9 Ack=4 Win=29200 Len=0
    ```

### Step4

* Description

    Combine function of step2(icmp ping) and step4(tcp server) using futures::future::Either.

    ```rust
            ...
            FramedRead::new(rd, BytesCodec::new())
            .filter_map(|bytes| {
                let tcpcon = Self::filter_with_tcp(&bytes).map(Either::A);
                let icmpbytes = Self::filter_with_icmp(bytes).map(Either::B);
                tcpcon.or(icmpbytes)
            })
            .map(|con_or_bytes| {
                match con_or_bytes{
                    Either::A(con) => Self::gen_tcp_packet(con),
                    Either::B(bytes) => Self::gen_ping_echo(bytes),
                }
            })
            .forward(writer)
            ...
    ```

### Step5

* Description

    implement ping via mpsc.

### Step6

* Description

    1.implement background process via reader-future and writer-future.    
    2.as application-level, empty "demo tcp server fn tcp_srv()" use libtcp as tcp stack.
    3.can accept multi tcp client on one tcp server, but does not implement tcpstream.

### Step6

* Description
    1.re-work Step6 avoid static Libtcp    

### Step7

* Description
    1.Deal with TCP.State LastAck,Closed while recive FIN
    2.Demo TCP Server for Connect/Read from Client/Closed from Client   