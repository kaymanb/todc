# atomic-register-docker-minikube

## Setup

Installed [`minikube`](https://minikube.sigs.k8s.io/docs/start/) and start a 
cluster with `minikube start`. 

## Build

Point your shell to `minikube`'s docker-daemon by running `minikube docker-env`
and following the instructions. Once this is done, the service can be built
with `docker-compose build`.

## Deploy

Run `kubectl apply -f deployment.yaml` to deploy `3` replicas of an atomic 
register. 


A Service is created for each instance of the register. To access instance `k` in `{0, 1, 2}` 
of the register, run `minikube service atomic-register-k --url`, which will
create a tunnel to the cluster and display a local `$URL` that the service can
be reached at. 


To read from the register:
```
curl $URL/register
```

To write to the register:
```
curl -d '"Hello, World!"' -X POST $URL/register
```

Any valid JSON can be written to the register:
```
curl -d '{"project": "todc", "crates": ["net", "mem", "utils"]}' -X POST $URL/register
```

To view logs for instance `k`, run `kubectl logs atomic-register-k`.

## Shutdown

To kill all services, run `kubectl delete -f deployment.yaml`. 
To stop `minikube`, run `minikube delete`. `
