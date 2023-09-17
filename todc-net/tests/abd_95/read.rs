use crate::simulate_servers;

#[test]
fn returns_current_value() {
    let (mut sim, replicas) = simulate_servers(2);
    sim.client("client", async move {
        let value = replicas[0].read().await.unwrap();
        assert_eq!(value, 0);
        Ok(())
    });
    sim.run().unwrap();
}

#[test]
fn returns_value_from_write_to_other_replica() {
    const VALUE: u32 = 123;
    let (mut sim, replicas) = simulate_servers(2);
    sim.client("client", async move {
        replicas[1].write(VALUE).await.unwrap();
        let value = replicas[0].read().await.unwrap();
        assert_eq!(value, VALUE);
        Ok(())
    });
    sim.run().unwrap();
}

#[test]
fn returns_even_if_half_of_neighbors_are_unreachable() {
    let (mut sim, replicas) = simulate_servers(3);
    sim.client("client", async move {
        turmoil::hold("client", "server-1");
        let value = replicas[0].read().await.unwrap();
        assert_eq!(value, 0);
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
        replicas[0].read().await.unwrap();
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
        replicas[0].read().await.unwrap();
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
        let result = replicas[0].read().await;
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("A majority of neighbors are offline"));
        Ok(())
    });

    sim.run().unwrap();
}
