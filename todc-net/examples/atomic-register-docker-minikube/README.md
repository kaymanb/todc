# atomic-register-docker-minikube

## Build

This example depends on the `todc-net` crate, which is installed from source. To 
build this example we need to set the target context to this crates root
directory, which can be done as follows:

```
docker build -t atomic-register-docker-minikube ../.. -f Dockerfile
```

## Run with Docker

Run this example with:

```
docker run --rm -p 3000:3000 --name atomic-register --env-file .env atomic-register-docker-minikube
```

To stop the example, run:

```
docker kill atomic-register
```

## Deploy with Minikube

Installed [`minikube`](https://minikube.sigs.k8s.io/docs/start/) and start a 
cluster with `minikube start`. 

Point your sheel to `minikube`'s docker-daemon by running `minikube docker-env`
and following the instructions. Make sure to re-build the image afterwards so 
that it is installed to `minikube`'s container registry.

(TODO: Write this better)
To deploy the service, run `kubectl apply -f deployment.yaml`.
A Service is created for each instance of the register. To access instance `k` 
of the register, run `minikube service atomic-register-k --url`, which will
create a tunnel to the cluster and expose the instance locally.
