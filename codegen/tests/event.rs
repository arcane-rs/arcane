use arcana_codegen::Event;

#[derive(Event)]
#[event(fqn = "wut", rev = 2)]
struct Wut {}
