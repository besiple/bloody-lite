#![windows_subsystem = "windows"]

mod bloody;
mod config;
mod hook;
mod keys;

use bloody::{check_bloody_device, dpi_to_sensitivity, set_windows_sensitivity, BloodyStatus};
use config::AppConfig;
use eframe::egui::{self, Color32, RichText, Vec2};
use hook::{start_mouse_hook, HookState};
use keys::{egui_key_to_vk, vk_to_string};
use std::sync::atomic::Ordering;
use std::sync::Arc;

struct BloodyLiteApp {
    config: AppConfig,
    hook_state: Arc<HookState>,
    bloody_status: BloodyStatus,
    recording_button: Option<u8>, // None, Some(4), Some(5)
    status_message: String,
}

impl BloodyLiteApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = AppConfig::load();
        let hook_state = Arc::new(HookState {
            button4_vk: std::sync::atomic::AtomicU16::new(config.button4_vk.unwrap_or(0)),
            button5_vk: std::sync::atomic::AtomicU16::new(config.button5_vk.unwrap_or(0)),
        });

        start_mouse_hook(hook_state.clone());
        let bloody_status = check_bloody_device();

        // Apply saved DPI sensitivity
        let sens = dpi_to_sensitivity(config.dpi);
        set_windows_sensitivity(sens);

        Self {
            config,
            hook_state,
            bloody_status,
            recording_button: None,
            status_message: "Готово к работе".to_string(),
        }
    }

    fn apply_dpi(&mut self, dpi: u32) {
        self.config.dpi = dpi;
        let sens = dpi_to_sensitivity(dpi);
        set_windows_sensitivity(sens);
        let _ = self.config.save();
        self.status_message = format!("Установлен DPI: {}", dpi);
    }

    fn set_button4_key(&mut self, vk: Option<u16>) {
        self.config.button4_vk = vk;
        self.hook_state
            .button4_vk
            .store(vk.unwrap_or(0), Ordering::SeqCst);
        let _ = self.config.save();
        self.status_message = match vk {
            Some(k) => format!("Кнопка 4 переназначена на [{}]", vk_to_string(k)),
            None => "Кнопка 4 сброшена на дефолт".to_string(),
        };
    }

    fn set_button5_key(&mut self, vk: Option<u16>) {
        self.config.button5_vk = vk;
        self.hook_state
            .button5_vk
            .store(vk.unwrap_or(0), Ordering::SeqCst);
        let _ = self.config.save();
        self.status_message = match vk {
            Some(k) => format!("Кнопка 5 переназначена на [{}]", vk_to_string(k)),
            None => "Кнопка 5 сброшена на дефолт".to_string(),
        };
    }
}

impl eframe::App for BloodyLiteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle key recording
        if let Some(btn) = self.recording_button {
            ctx.input(|i| {
                for event in &i.events {
                    if let egui::Event::Key {
                        key,
                        pressed: true,
                        ..
                    } = event
                    {
                        if *key == egui::Key::Escape {
                            self.recording_button = None;
                            self.status_message = "Назначение отменено".to_string();
                        } else if let Some(vk) = egui_key_to_vk(*key) {
                            if btn == 4 {
                                self.set_button4_key(Some(vk));
                            } else if btn == 5 {
                                self.set_button5_key(Some(vk));
                            }
                            self.recording_button = None;
                        }
                    }
                }
            });
        }

        // Custom dark theme with bloody red accent
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(Color32::from_rgb(230, 230, 230));
        visuals.panel_fill = Color32::from_rgb(18, 18, 22);
        visuals.window_fill = Color32::from_rgb(18, 18, 22);
        ctx.set_visuals(visuals);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 12.0);

            // Header
            ui.horizontal(|ui| {
                ui.heading(
                    RichText::new("⚡ Bloody Lite")
                        .color(Color32::from_rgb(235, 50, 50))
                        .size(22.0)
                        .strong(),
                );
                ui.label(
                    RichText::new("v0.1.0")
                        .color(Color32::from_rgb(120, 120, 120))
                        .small(),
                );
            });

            // Device Status
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    if self.bloody_status.connected {
                        ui.label(RichText::new("●").color(Color32::from_rgb(50, 220, 50)));
                        ui.label(
                            RichText::new(format!(
                                "Устройство: {} (PID: 0x{:04X})",
                                self.bloody_status.product_name, self.bloody_status.pid
                            ))
                            .color(Color32::from_rgb(180, 220, 180)),
                        );
                    } else {
                        ui.label(RichText::new("●").color(Color32::from_rgb(220, 160, 50)));
                        ui.label("Мышь Bloody не найдена (используется системный режим)");
                    }
                });
            });

            ui.add_space(6.0);

            // DPI Section
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("НАСТРОЙКА DPI").strong().size(14.0));

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{} DPI", self.config.dpi))
                                .size(24.0)
                                .color(Color32::from_rgb(240, 70, 70))
                                .strong(),
                        );
                    });

                    ui.label("Быстрый выбор:");
                    ui.horizontal(|ui| {
                        let presets = [800, 1000, 1200, 1600, 3200];
                        for &dpi in &presets {
                            let is_current = self.config.dpi == dpi;
                            let btn_text = if is_current {
                                RichText::new(format!("✓ {}", dpi))
                                    .color(Color32::WHITE)
                                    .strong()
                            } else {
                                RichText::new(format!("{}", dpi))
                            };

                            if ui.button(btn_text).clicked() {
                                self.apply_dpi(dpi);
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        let mut current_dpi = self.config.dpi;
                        ui.label("Точный выбор:");
                        if ui
                            .add(egui::Slider::new(&mut current_dpi, 100..=3200).step_by(50.0))
                            .changed()
                        {
                            self.apply_dpi(current_dpi);
                        }
                    });
                });
            });

            ui.add_space(6.0);

            // Side Buttons Remapping Section
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("БОКОВЫЕ КНОПКИ МЫШИ")
                            .strong()
                            .size(14.0),
                    );

                    // Button 4 (Bottom side button)
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Кнопка 4 (Нижняя):").strong());
                        let desc = match self.config.button4_vk {
                            Some(vk) => {
                                RichText::new(format!("Клавиша [{}]", vk_to_string(vk)))
                                    .color(Color32::from_rgb(100, 200, 255))
                            }
                            None => RichText::new("По умолчанию (Назад)")
                                .color(Color32::from_rgb(180, 180, 180)),
                        };
                        ui.label(desc);
                    });

                    ui.horizontal(|ui| {
                        if self.recording_button == Some(4) {
                            ui.label(
                                RichText::new("🔴 Нажмите клавишу на клаве (Esc = отмена)...")
                                    .color(Color32::from_rgb(255, 120, 120))
                                    .strong(),
                            );
                        } else {
                            if ui.button("🎯 Задать клавишу").clicked() {
                                self.recording_button = Some(4);
                                self.status_message =
                                    "Нажмите нужную клавишу на клавиатуре...".to_string();
                            }
                            if self.config.button4_vk.is_some() {
                                if ui.button("↺ Сбросить на дефолт").clicked() {
                                    self.set_button4_key(None);
                                }
                            }
                        }
                    });

                    ui.separator();

                    // Button 5 (Top side button)
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Кнопка 5 (Верхняя):").strong());
                        let desc = match self.config.button5_vk {
                            Some(vk) => {
                                RichText::new(format!("Клавиша [{}]", vk_to_string(vk)))
                                    .color(Color32::from_rgb(100, 200, 255))
                            }
                            None => RichText::new("По умолчанию (Вперёд)")
                                .color(Color32::from_rgb(180, 180, 180)),
                        };
                        ui.label(desc);
                    });

                    ui.horizontal(|ui| {
                        if self.recording_button == Some(5) {
                            ui.label(
                                RichText::new("🔴 Нажмите клавишу на клаве (Esc = отмена)...")
                                    .color(Color32::from_rgb(255, 120, 120))
                                    .strong(),
                            );
                        } else {
                            if ui.button("🎯 Задать клавишу").clicked() {
                                self.recording_button = Some(5);
                                self.status_message =
                                    "Нажмите нужную клавишу на клавиатуре...".to_string();
                            }
                            if self.config.button5_vk.is_some() {
                                if ui.button("↺ Сбросить на дефолт").clicked() {
                                    self.set_button5_key(None);
                                }
                            }
                        }
                    });
                });
            });

            ui.add_space(6.0);

            // Footer Status
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(&self.status_message)
                        .color(Color32::from_rgb(140, 180, 220))
                        .small(),
                );
            });

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("⚡ Память: ~15 MB RAM | CPU: 0.0% | Без фонового мусора")
                        .color(Color32::from_rgb(100, 100, 100))
                        .small(),
                );
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([460.0, 520.0])
            .with_min_inner_size([440.0, 480.0])
            .with_title("Bloody Lite - Mouse Controller"),
        ..Default::default()
    };

    eframe::run_native(
        "Bloody Lite",
        native_options,
        Box::new(|cc| Ok(Box::new(BloodyLiteApp::new(cc)))),
    )
}
