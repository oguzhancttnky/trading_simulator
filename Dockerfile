FROM rust:1.84.0

WORKDIR /usr/src/trading_simulator_app
COPY . .

RUN cargo install --path .

CMD ["trading_simulator_app"]