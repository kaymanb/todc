# atomic-register-docker-minikube

## Build

This example depends on the `todc-net` crate, which is installed from source. To 
build this example we need to set the target context to this crates root
directory, which can be done as follows:

```
docker build -t atomic-register-docker-minikube ../.. -f Dockerfile
```

## Run

Run this example with:

```
docker run --rm -p 3000:3000 --name ardm atomic-register-docker-minikub
```

To stop the example, run:

```
docker kill ardm
```
