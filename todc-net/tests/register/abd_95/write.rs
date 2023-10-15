use crate::register::abd_95::common::simulate_servers;

#[test]
fn sets_value_of_requested_replica() {
    let (mut sim, replicas) = simulate_servers(2);
    sim.client("client", async move {
        replicas[0].write(123).await.unwrap();
        let value = replicas[0].read().await.unwrap();
        assert_eq!(value, 123);
        Ok(())
    });
    sim.run().unwrap();
}

#[test]
fn sets_value_of_all_other_replicas() {
    const NUM_REPLICAS: usize = 3;
    const VALUE: u32 = 123;
    let (mut sim, replicas) = simulate_servers(NUM_REPLICAS);
    sim.client("client", async move {
        replicas[0].write(VALUE).await.unwrap();
        for i in (0..NUM_REPLICAS).rev() {
            let value = replicas[i].read().await.unwrap();
            assert_eq!(value, VALUE);
        }
        Ok(())
    });
    sim.run().unwrap();
}

#[test]
fn returns_even_if_half_of_neighbors_are_unreachable() {
    let (mut sim, replicas) = simulate_servers(3);
    sim.client("client", async move {
        turmoil::hold("client", "server-1");
        replicas[0].write(123).await.unwrap();
        let value = replicas[0].read().await.unwrap();
        assert_eq!(value, 123);
        Ok(())
    });
    sim.run().unwrap();
}

#[test]
fn hangs_if_more_than_half_of_neighbors_are_unreachable() {
    let (mut sim, replicas) = simulate_servers(3);
    sim.client("client", async move {
        turmoil::hold("client", "server-1");
        turmoil::hold("client", "server-2");
        replicas[0].write(123).await.unwrap();
        Ok(())
    });

    assert!(sim
        .run()
        .unwrap_err()
        .to_string()
        .contains("Ran for 10s without completing"))
}

#[test]
fn returns_even_if_half_of_neighbors_are_offline() {
    let (mut sim, replicas) = simulate_servers(3);
    sim.client("client", async move {
        turmoil::partition("client", "server-1");
        replicas[0].write(123).await.unwrap();
        Ok(())
    });

    sim.run().unwrap();
}

#[test]
fn raises_error_if_more_than_half_of_neighbors_are_offline() {
    let (mut sim, replicas) = simulate_servers(3);
    sim.client("client", async move {
        turmoil::partition("client", "server-1");
        turmoil::partition("client", "server-2");
        let result = replicas[0].write(123).await;
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("A majority of neighbors are offline"));
        Ok(())
    });

    sim.run().unwrap();
}
