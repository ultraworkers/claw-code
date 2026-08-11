use agents::make_agent_id;

#[test]
fn make_agent_id_is_unique_under_burst() {
    let mut ids = std::collections::HashSet::new();
    for _ in 0..1000 {
        let id = make_agent_id();
        assert!(ids.insert(id.clone()), "duplicate id {id}");
    }
}
