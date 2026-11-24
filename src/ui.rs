use gpui::{
    App, Application, Bounds, ClickEvent, Context, PathPromptOptions, SharedString, Window,
    WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};
use std::{path::PathBuf, process::ExitCode};

#[derive(Default)]
struct Patcher {
    folder: Option<PathBuf>,
    report: Vec<String>,
    busy: bool,
    confirm_restore: bool,
}

impl Patcher {
    fn browse(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let selection = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Selecionar pasta da extensão".into()),
        });
        self.busy = true;
        self.confirm_restore = false;
        cx.notify();
        cx.spawn(async move |view, cx| {
            let result = selection.await;
            let _ = view.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(Ok(Some(paths))) => {
                        this.folder = paths.into_iter().next();
                        this.report.clear();
                    }
                    Ok(Ok(None)) => {}
                    Ok(Err(error)) => this.report = vec![format!("[ERRO] {error}")],
                    Err(error) => this.report = vec![format!("[ERRO] {error}")],
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn execute(&mut self, restore: bool, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(path) = self.folder.clone() else {
            return;
        };
        self.busy = true;
        self.confirm_restore = false;
        self.report = vec!["Validando arquivos…".into()];
        cx.notify();

        let work = cx.background_executor().spawn(async move {
            let mut report = Vec::new();
            if let Err(error) = crate::operate(&path, restore, &mut |line| report.push(line)) {
                report.push(format!("[ERRO] {error}"));
            }
            report
        });
        cx.spawn(async move |view, cx| {
            let report = work.await;
            let _ = view.update(cx, |this, cx| {
                this.report = report;
                this.busy = false;
                cx.notify();
            });
        })
        .detach();
    }
}

fn button(
    id: &'static str,
    label: &'static str,
    accent: bool,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let color = if accent { 0xf48255 } else { 0x303747 };
    let element = div()
        .id(id)
        .px_4()
        .py_2()
        .rounded_md()
        .bg(rgb(if enabled { color } else { 0x252a33 }))
        .text_color(rgb(if !enabled {
            0x929aa9
        } else if accent {
            0x1a1412
        } else {
            0xe5e9ef
        }))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .child(label);
    if enabled {
        element
            .cursor_pointer()
            .hover(move |style| style.bg(rgb(if accent { 0xff9b70 } else { 0x3d4658 })))
    } else {
        element
    }
}

impl Render for Patcher {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ready = self.folder.is_some() && !self.busy;
        let path = self
            .folder
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "Selecione a pasta que contém katamari.js e manifest.json.".into());
        let report = if self.report.is_empty() {
            vec!["O relatório da operação aparecerá aqui.".into()]
        } else {
            self.report.clone()
        };
        let actions = div().flex().flex_wrap().gap_3().items_center();
        let actions =
            if self.confirm_restore {
                actions
                    .child("Restaurar um backup antigo compatível?")
                    .child(
                        button("confirm-restore", "Restaurar", true, ready)
                            .on_click(cx.listener(|this, _, _, cx| this.execute(true, cx))),
                    )
                    .child(button("cancel-restore", "Cancelar", false, ready).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.confirm_restore = false;
                            cx.notify();
                        }),
                    ))
            } else {
                actions
                    .child(
                        button("apply", "Aplicar patch", true, ready)
                            .on_click(cx.listener(|this, _, _, cx| this.execute(false, cx))),
                    )
                    .child(
                        button("restore", "Restaurar backup antigo", false, ready).on_click(
                            cx.listener(|this, _, _, cx| {
                                if this.folder.is_some() && !this.busy {
                                    this.confirm_restore = true;
                                    cx.notify();
                                }
                            }),
                        ),
                    )
            };

        div().id("patcher").size_full().overflow_y_scroll().bg(rgb(0x141820))
            .text_color(rgb(0xe7ecf4)).p_6().flex().flex_col().gap_5()
            .child(div().text_2xl().font_weight(gpui::FontWeight::SEMIBOLD).child("CrOptix Patcher"))
            .child(div().bg(rgb(0x202631)).rounded_lg().p_4().flex().flex_col().gap_3()
                .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child("Pasta da extensão"))
                .child(div().text_sm().text_color(rgb(0xb7c2d3)).child(path))
                .child(div().child(button("browse", "Escolher pasta", false, !self.busy)
                    .on_click(cx.listener(Self::browse)))))
            .child(actions)
            .child(div().text_sm().text_color(rgb(0xb7c2d3)).child(
                "Inclui PiP com legendas, próximo episódio, ajustes de avanço e CC automático em dublagens. A aplicação não cria backups."
            ))
            .child(div().flex().flex_col().flex_1().min_h(px(150.0)).gap_2()
                .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child("Relatório"))
                .child(div().id("report").flex_1().overflow_y_scroll().rounded_lg()
                    .bg(rgb(0x0f131a)).p_3().text_sm()
                    .children(report.into_iter().map(|line| {
                        let color = if line.starts_with("[ERRO]") { 0xff9a8d } else { 0xc5cfde };
                        div().text_color(rgb(color)).child(SharedString::from(line))
                    }))))
    }
}

pub fn run() -> ExitCode {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(760.0), px(560.0)), cx);
        let result = cx.open_window(
            WindowOptions {
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("CrOptix Patcher".into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|_| Patcher::default());
                let weak = view.downgrade();
                window.on_window_should_close(cx, move |_, cx| {
                    weak.upgrade().is_none_or(|view| !view.read(cx).busy)
                });
                view
            },
        );
        match result {
            Ok(_) => cx.activate(true),
            Err(error) => {
                eprintln!("Não foi possível abrir o CrOptix Patcher: {error}");
                cx.quit();
            }
        }
    });
    ExitCode::SUCCESS
}
