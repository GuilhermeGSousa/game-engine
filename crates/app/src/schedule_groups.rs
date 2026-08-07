use ecs::system::schedule::ScheduleLabel;

macro_rules! define_schedule_label {
    ($label_trait_name:ident) => {
        #[derive(Clone, PartialEq, Eq, Hash, Debug)]
        pub struct $label_trait_name;

        impl ScheduleLabel for $label_trait_name {
            fn dyn_clone(&self) -> Box<dyn ScheduleLabel> {
                Box::new(self.clone())
            }
        }
    };
}

define_schedule_label!(Main);
define_schedule_label!(Startup);
define_schedule_label!(Update);
define_schedule_label!(FixedUpdate);
define_schedule_label!(LateUpdate);
define_schedule_label!(LateFixedUpdate);
define_schedule_label!(Render);
define_schedule_label!(LateRender);
