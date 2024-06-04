FROM rabbitmq:3.13
RUN apt update && apt install -y libssl-dev pkg-config wget
RUN wget https://github.com/questdb/questdb/releases/download/8.0.0/questdb-8.0.0-rt-linux-amd64.tar.gz && \
    tar -xvf questdb-8.0.0-rt-linux-amd64.tar.gz && \
    ./questdb-8.0.0-rt-linux-amd64/bin/questdb.sh start
