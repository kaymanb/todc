# atomic-register-hyper

This is a minimal example for running one instance of [`AtomicRegister`]
locally.

To start the instance:
```
cargo run
```

To write to the regiser:
```
curl -d 'Hello, World!' -X POST http://localhost:3000/register
````

To read the register:
```
curl http://localhost:3000/register
```
