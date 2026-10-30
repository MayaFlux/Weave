//! Stage identity and progress formatting used by both front ends and native plans.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    Prerequisites,
    Framework,
    Dependencies,
    Environment,
    Resources,
    Completion,
}
impl Stage {
    pub fn report(self, detail: &str, log: &impl Fn(&str)) {
        let number = match self {
            Self::Prerequisites => 1,
            Self::Framework => 2,
            Self::Dependencies => 3,
            Self::Environment => 4,
            Self::Resources => 5,
            Self::Completion => 6,
        };
        log(&format!("Step {number}/6: {detail}"));
    }
}
