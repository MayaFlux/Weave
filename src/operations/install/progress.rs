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
    pub fn title(self) -> &'static str {
        match self {
            Self::Prerequisites => "System check",
            Self::Framework => "Install MayaFlux",
            Self::Dependencies => "Dependencies",
            Self::Environment => "Environment",
            Self::Resources => "Templates / CLI",
            Self::Completion => "Complete",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Prerequisites => "system",
            Self::Framework => "install",
            Self::Dependencies => "dependencies",
            Self::Environment => "environment",
            Self::Resources => "resources",
            Self::Completion => "complete",
        }
    }

    pub fn report(self, detail: &str, log: &impl Fn(&str)) {
        log(&Event::Step(self, detail.into())
            .encode()
            .expect("step is encodable"));
    }
}

/// A single-line envelope preserves typed events through the elevated worker's pipe.
/// Ordinary command output is never interpreted as a human-readable step heading.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Step(Stage, String),
    Transfer(Option<f32>),
    Output(String),
}

impl Event {
    pub fn encode(&self) -> Option<String> {
        match self {
            Self::Step(stage, detail) => Some(format!(
                "@weave-progress:{}",
                serde_json::json!({"stage": stage.key(), "detail": detail})
            )),
            Self::Transfer(value) => Some(format!(
                "@weave-progress:{}",
                serde_json::json!({"transfer": value})
            )),
            Self::Output(_) => None,
        }
    }

    pub fn decode(line: &str) -> Self {
        let parsed = || {
            let value: serde_json::Value =
                serde_json::from_str(line.strip_prefix("@weave-progress:")?).ok()?;
            if let Some(transfer) = value.get("transfer") {
                if transfer.is_null() {
                    return Some(Self::Transfer(None));
                }
                let fraction = transfer.as_f64()?;
                return (fraction.is_finite() && (0.0..=1.0).contains(&fraction))
                    .then_some(Self::Transfer(Some(fraction as f32)));
            }
            let key = value.get("stage")?.as_str()?;
            let stage = [
                Stage::Prerequisites,
                Stage::Framework,
                Stage::Dependencies,
                Stage::Environment,
                Stage::Resources,
                Stage::Completion,
            ]
            .into_iter()
            .find(|s| s.key() == key)?;
            Some(Self::Step(stage, value.get("detail")?.as_str()?.into()))
        };
        parsed().unwrap_or_else(|| Self::Output(line.into()))
    }

    pub fn text(&self) -> String {
        match self {
            Self::Step(stage, detail) => format!("{}: {detail}", stage.title()),
            Self::Transfer(Some(value)) => format!("Download: {:.0}%", value * 100.0),
            Self::Transfer(None) => "Download transfer ended.".into(),
            Self::Output(line) => line.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Platform {
    Windows,
    Macos,
    Linux,
}

impl Platform {
    pub fn current() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self::Windows
        }
        #[cfg(target_os = "macos")]
        {
            Self::Macos
        }
        #[cfg(target_os = "linux")]
        {
            Self::Linux
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Pending,
    Running,
    Complete,
    Failed,
}

pub struct Step {
    pub stage: Stage,
    pub status: Status,
    pub output: String,
}

pub struct Progress {
    pub steps: Vec<Step>,
    pub active: usize,
    pub summary: String,
    pub fraction: Option<f32>,
}

impl Progress {
    pub fn new(platform: Platform) -> Self {
        let stages: &[Stage] = match platform {
            Platform::Windows => &[
                Stage::Prerequisites,
                Stage::Framework,
                Stage::Dependencies,
                Stage::Environment,
                Stage::Resources,
            ],
            Platform::Macos => &[
                Stage::Prerequisites,
                Stage::Framework,
                Stage::Environment,
                Stage::Resources,
            ],
            Platform::Linux => &[Stage::Prerequisites, Stage::Framework, Stage::Resources],
        };
        Self {
            steps: stages
                .iter()
                .map(|stage| Step {
                    stage: *stage,
                    status: Status::Pending,
                    output: String::new(),
                })
                .collect(),
            active: 0,
            summary: String::new(),
            fraction: None,
        }
    }

    pub fn receive(&mut self, event: Event) {
        match event {
            Event::Step(stage, detail) => {
                self.fraction = None;
                if let Some(index) = self.steps.iter().position(|s| s.stage == stage) {
                    if index > self.active {
                        for step in &mut self.steps[..index] {
                            step.status = Status::Complete;
                        }
                    }
                    self.active = index;
                    self.steps[index].status = Status::Running;
                }
                self.append(&detail);
            }
            Event::Output(line) => {
                if self.steps[self.active].status == Status::Pending {
                    self.steps[self.active].status = Status::Running;
                }
                self.append(&line);
            }
            Event::Transfer(value) => self.fraction = value,
        }
    }

    fn append(&mut self, line: &str) {
        let output = &mut self.steps[self.active].output;
        if !output.is_empty() {
            output.push_str("\n\n");
        }
        output.push_str(line);
        // Keep the view bounded during verbose package-manager output. Native logs
        // remain independent of this display buffer.
        if output.len() > 128 * 1024 {
            let mut boundary = output.len() - 96 * 1024;
            while !output.is_char_boundary(boundary) {
                boundary += 1;
            }
            output.drain(..boundary);
        }
    }

    pub fn finish(&mut self, result: Result<String, String>) {
        self.fraction = None;
        match result {
            Ok(message) => {
                for step in &mut self.steps {
                    step.status = Status::Complete;
                }
                self.summary = message;
            }
            Err(error) => {
                self.steps[self.active].status = Status::Failed;
                self.summary = error;
            }
        }
        self.append(&self.summary.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_survives_line_transport_without_parsing_human_output() {
        let event = Event::Step(Stage::Framework, "Downloading\nMayaFlux: 50%".into());
        let wire = event.encode().unwrap();
        assert!(!wire.contains('\n'));
        assert_eq!(Event::decode(&wire), event);
        assert_eq!(
            Event::decode("Step 3/6: ordinary output"),
            Event::Output("Step 3/6: ordinary output".into())
        );
        assert!(matches!(
            Event::decode("@weave-progress:{broken"),
            Event::Output(_)
        ));
    }

    #[test]
    fn platform_pages_only_include_applicable_work() {
        assert_eq!(Progress::new(Platform::Windows).steps.len(), 5);
        assert_eq!(
            Progress::new(Platform::Macos)
                .steps
                .iter()
                .map(|s| s.stage)
                .collect::<Vec<_>>(),
            vec![
                Stage::Prerequisites,
                Stage::Framework,
                Stage::Environment,
                Stage::Resources
            ]
        );
        assert_eq!(
            Progress::new(Platform::Linux)
                .steps
                .iter()
                .map(|s| s.stage)
                .collect::<Vec<_>>(),
            vec![Stage::Prerequisites, Stage::Framework, Stage::Resources]
        );
    }

    #[test]
    fn failure_preserves_step_output_and_does_not_complete_pending_steps() {
        let mut state = Progress::new(Platform::Windows);
        state.receive(Event::Step(Stage::Prerequisites, "Checking tools".into()));
        state.receive(Event::Output("UAC accepted".into()));
        state.receive(Event::Step(Stage::Framework, "Installing".into()));
        state.receive(Event::Output("download interrupted".into()));
        state.finish(Err("Retry installation".into()));
        assert_eq!(state.steps[0].status, Status::Complete);
        assert_eq!(state.steps[1].status, Status::Failed);
        assert_eq!(state.steps[2].status, Status::Pending);
        assert!(state.steps[0].output.contains("UAC accepted"));
        assert!(state.steps[1].output.contains("download interrupted"));
        assert!(state.steps[1].output.contains("Retry installation"));
        let retry = Progress::new(Platform::Windows);
        assert_eq!(retry.steps[1].status, Status::Pending);
    }

    #[test]
    fn completion_signal_does_not_hide_a_later_worker_failure() {
        let mut state = Progress::new(Platform::Linux);
        state.receive(Event::Step(Stage::Framework, "Installing".into()));
        state.receive(Event::Step(Stage::Resources, "Checking resources".into()));
        state.receive(Event::Step(Stage::Completion, "Finished".into()));
        state.finish(Err("Worker failed to exit".into()));
        assert_eq!(state.steps[2].status, Status::Failed);
    }

    #[test]
    fn measured_transfer_progress_resets_before_extraction_and_on_failure() {
        let mut state = Progress::new(Platform::Windows);
        let event = Event::Transfer(Some(0.42));
        assert_eq!(Event::decode(&event.encode().unwrap()), event);
        state.receive(event);
        assert_eq!(state.fraction, Some(0.42));
        state.receive(Event::Transfer(None));
        assert_eq!(state.fraction, None);
        state.receive(Event::Transfer(Some(0.5)));
        state.finish(Err("Download failed".into()));
        assert_eq!(state.fraction, None);
        for malformed in [
            "@weave-progress:{\"transfer\":3}",
            "@weave-progress:{\"transfer\":\"NaN\"}",
            "@weave-progress:{\"stage\":\"unknown\",\"detail\":\"ignored\"}",
        ] {
            assert!(matches!(Event::decode(malformed), Event::Output(_)));
        }
    }
}
