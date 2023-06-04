# atomic-register-docker-minikube

## Setup

Installed [`minikube`](https://minikube.sigs.k8s.io/docs/start/) and start a 
cluster with `minikube start`. 

## Build

Point your sheel to `minikube`'s docker-daemon by running `minikube docker-env`
and following the instructions. Once this is done, the service can be built
with `docker-compose build`.

## Deploy

(TODO: Write this better)
To deploy the service, run `kubectl apply -f deployment.yaml`.

A Service is created for each instance of the register. To access instance `k` 
of the register, run `minikube service atomic-register-k --url`, which will
create a tunnel to the cluster and display a local URL that the service can
be reached at.

To read from the register:
```
curl http://127.0.0.1:54478/register
```

To write to the register:
```
curl -d '"Hello, World!"' -X POST http://127.0.0.1:54478/register
```

## Shutdown

To kill all services, run `kubectl delete -f deployment.yaml`. 
To stop `minikube`, run `minikube delete`. `
