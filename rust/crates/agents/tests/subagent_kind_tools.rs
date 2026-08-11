use agents::SubagentKind;

#[test]
fn general_purpose_has_a_maximal_tool_set() {
    let tools = SubagentKind::GeneralPurpose.allowed_tools();
    assert!(!tools.is_empty(), "GeneralPurpose should keep its broad tool set");
    assert!(tools.contains("bash"));
    assert!(tools.contains("new_file"));
}

#[test]
fn custom_subagent_is_fail_closed() {
    let tools = SubagentKind::Custom("anything-here".to_string()).allowed_tools();
    assert!(
        tools.is_empty(),
        "Custom subagents must be fail-closed; got {tools:?}",
    );
}

#[test]
fn custom_subagent_empty_regardless_of_name() {
    let a = SubagentKind::Custom("foo".to_string()).allowed_tools();
    let b = SubagentKind::Custom("general-purpose".to_string()).allowed_tools();
    let c = SubagentKind::Custom("general".to_string()).allowed_tools();
    assert!(a.is_empty());
    assert!(b.is_empty());
    assert!(c.is_empty());
}

#[test]
fn explore_remains_read_only() {
    let tools = SubagentKind::Explore.allowed_tools();
    assert!(tools.contains("read_file"));
    assert!(!tools.contains("bash"));
    assert!(!tools.contains("new_file"));
}
