FROM rust:1.69 as builder

# Because todc-net is installed from source, we copy the entire
# crate (including this example) to the container.
# into the container.
WORKDIR /usr/src/todc/todc-net
COPY . .

WORKDIR ./examples/atomic-register-docker-minikube
RUN cargo install --path .

FROM debian:bullseye-slim
# RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/atomic-register-docker-minikube /usr/local/bin/atomic-register-docker-minikube
CMD ["atomic-register-docker-minikube"]

