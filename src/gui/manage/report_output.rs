//! Readable GUI presentation for Run & Report's plain text output.

#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum Section {
    #[default]
    Start,
    Previous,
    Setup,
    Application,
    Result,
    Files,
}

/// Adds section headings without changing the script output or saved logs.
#[derive(Default)]
pub(super) struct ReportOutput {
    section: Section,
    has_lines: bool,
}

impl ReportOutput {
    /// Formats one streamed line while retaining its place in the report.
    pub fn push(&mut self, line: &str) -> String {
        let next = match self.section {
            Section::Start if line.starts_with("Previous log saved:") => Some(Section::Previous),
            Section::Start | Section::Previous if line.starts_with("Executable:") => {
                Some(Section::Setup)
            }
            Section::Start | Section::Setup if line.starts_with("Reproduce the problem,") => {
                Some(Section::Application)
            }
            Section::Setup | Section::Application if line.starts_with("Application exit code:") => {
                Some(Section::Result)
            }
            Section::Setup | Section::Application | Section::Result
                if line.starts_with("Console log:") || line.starts_with("Application report:") =>
            {
                Some(Section::Files)
            }
            _ => None,
        };

        let Some(next) = next else {
            self.has_lines = true;
            return line.to_owned();
        };

        self.section = next;
        let title = match next {
            Section::Previous => "Previous reports",
            Section::Setup => "Run setup",
            Section::Application => "Application output",
            Section::Result => "Run result",
            Section::Files => "Report files",
            Section::Start => unreachable!(),
        };
        let spacing = if self.has_lines { "\n" } else { "" };
        self.has_lines = true;
        format!("{spacing}{title}\n\n{line}")
    }
}

/// Formats completed output with the same sections shown during streaming.
pub(super) fn format_run_report_output(raw: &str) -> String {
    let mut output = ReportOutput::default();
    raw.lines()
        .map(|line| output.push(line))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{ReportOutput, format_run_report_output};

    #[test]
    fn report_sections_separate_setup_application_result_and_files() {
        let raw = "Previous log saved: old.txt\nExecutable: /project/app\nProject folder: /project\nReport configuration: /config.json\nReproduce the problem, then close the application to finish collecting logs.\nMayaFlux initialized\nApplication exit code: 0\nConsole log: /project/mayaflux_run.txt\nApplication report: /project/data/log/mayaflux_report.log";

        let formatted = format_run_report_output(raw);

        for section in [
            "Previous reports",
            "Run setup",
            "Application output",
            "Run result",
            "Report files",
        ] {
            assert!(
                formatted.contains(section),
                "missing {section}: {formatted}"
            );
        }
        assert!(formatted.contains("Report configuration: /config.json\n\nApplication output"));
        assert!(formatted.contains("MayaFlux initialized\n\nRun result"));
        assert!(formatted.contains("Application exit code: 0\n\nReport files"));

        let mut live = ReportOutput::default();
        let streamed = raw
            .lines()
            .map(|line| live.push(line))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(streamed, formatted);
    }
}
