slint::slint! {
    export component MainWindow inherits Window {
        title: "Weave";
        width: 720px;
        height: 480px;

        Text {
            text: "Weave";
            font-size: 28px;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    MainWindow::new()?.run()
}
