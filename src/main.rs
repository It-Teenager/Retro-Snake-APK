use macroquad::audio::{load_sound_from_bytes, play_sound_once, Sound};
use macroquad::prelude::*;
use std::fs;

const GRID_W: i32 = 20;
const GRID_H: i32 = 18;

const NOKIA_BG: Color = Color::from_rgba(139, 172, 15, 255);
const NOKIA_GRID: Color = Color::from_rgba(125, 158, 12, 255);
const NOKIA_DARK: Color = Color::from_rgba(15, 56, 15, 255);
const SNAKE_BODY: Color = Color::from_rgba(110, 235, 52, 255);
const SNAKE_HEAD: Color = Color::from_rgba(165, 255, 80, 255);
const SNAKE_DYING: Color = Color::from_rgba(200, 255, 140, 255);
const APPLE_RED: Color = Color::from_rgba(225, 45, 45, 255);
const GOLD_CUP: Color = Color::from_rgba(245, 195, 30, 255);

#[derive(PartialEq, Clone, Copy)]
enum State { Menu, Play, Dying, Over }

#[derive(PartialEq, Clone, Copy)]
enum Dir { Up, Down, Left, Right }

fn window_conf() -> Conf {
    Conf {
        window_title: "Retro Snake".to_owned(),
        window_width: 420,
        window_height: 760,
        fullscreen: cfg!(target_os = "android"),
        window_resizable: true,
        ..Default::default()
    }
}

fn make_retro_wav(notes: &[(f32, f32)]) -> Vec<u8> {
    let rate = 22050u32;
    let mut samples = Vec::new();
    for &(freq, dur) in notes {
        let count = (rate as f32 * dur) as usize;
        for i in 0..count {
            let t = i as f32 / rate as f32;
            let wave = if (t * freq * 2.0 * std::f32::consts::PI).sin() >= 0.0 { 0.25 } else { -0.25 };
            samples.push((wave * 32767.0) as i16);
        }
    }
    let data_len = (samples.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&rate.to_le_bytes());
    wav.extend_from_slice(&(rate * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for s in samples { wav.extend_from_slice(&s.to_le_bytes()); }
    wav
}

fn get_pointer() -> (Vec2, bool, bool) {
    if let Some(t) = touches().first() {
        return (t.position, true, t.phase == TouchPhase::Started);
    }
    let (mx, my) = mouse_position();
    (vec2(mx, my), is_mouse_button_down(MouseButton::Left), is_mouse_button_pressed(MouseButton::Left))
}

fn draw_ctrl_btn(rect: Rect, arrow: Dir, ptr: Vec2, down: bool, pressed: bool) -> bool {
    let inside = rect.contains(ptr);
    let active = inside && down;
    let offset = if active { rect.h * 0.05 } else { 0.0 };

    draw_rectangle(rect.x + 3.0, rect.y + 4.0, rect.w, rect.h, Color::from_rgba(10, 35, 10, 180));
    draw_rectangle(rect.x, rect.y + offset, rect.w, rect.h, if active { SNAKE_HEAD } else { NOKIA_DARK });
    draw_rectangle_lines(rect.x, rect.y + offset, rect.w, rect.h, 3.0, if active { NOKIA_DARK } else { SNAKE_BODY });

    let c = vec2(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0 + offset);
    let r = rect.h.min(rect.w) * 0.24;
    let col = if active { NOKIA_DARK } else { SNAKE_HEAD };
    match arrow {
        Dir::Up => draw_triangle(vec2(c.x, c.y - r), vec2(c.x - r, c.y + r * 0.8), vec2(c.x + r, c.y + r * 0.8), col),
        Dir::Down => draw_triangle(vec2(c.x, c.y + r), vec2(c.x - r, c.y - r * 0.8), vec2(c.x + r, c.y - r * 0.8), col),
        Dir::Left => draw_triangle(vec2(c.x - r, c.y), vec2(c.x + r * 0.8, c.y - r), vec2(c.x + r * 0.8, c.y + r), col),
        Dir::Right => draw_triangle(vec2(c.x + r, c.y), vec2(c.x - r * 0.8, c.y - r), vec2(c.x - r * 0.8, c.y + r), col),
    }
    inside && pressed
}

fn draw_menu_btn(rect: Rect, text: &str, ptr: Vec2, down: bool, pressed: bool) -> bool {
    let hov = rect.contains(ptr);
    let active = hov && down;
    draw_rectangle(rect.x + 4.0, rect.y + 4.0, rect.w, rect.h, Color::from_rgba(10, 35, 10, 190));
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, if hov { SNAKE_HEAD } else { NOKIA_DARK });
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 3.0, NOKIA_DARK);
    let font_sz = (rect.h * 0.55) as u16;
    let d = measure_text(text, None, font_sz, 1.0);
    draw_text(text, rect.x + (rect.w - d.width) / 2.0, rect.y + (rect.h + d.height) / 2.0 - 3.0, font_sz as f32, if hov { NOKIA_DARK } else { NOKIA_BG });
    hov && (pressed || active)
}

fn spawn_apple(snake: &[(i32, i32)]) -> (i32, i32) {
    loop {
        let p = (rand::gen_range(0, GRID_W), rand::gen_range(0, GRID_H));
        if !snake.contains(&p) { return p; }
    }
}
#[macroquad::main(window_conf)]
async fn main() {
    rand::srand(macroquad::miniquad::date::now() as u64);

    let sfx_eat: Sound = load_sound_from_bytes(&make_retro_wav(&[(880.0, 0.06)])).await.unwrap();
    let sfx_rec: Sound = load_sound_from_bytes(&make_retro_wav(&[(659.0, 0.07), (880.0, 0.07), (1318.0, 0.14)])).await.unwrap();
    let sfx_die: Sound = load_sound_from_bytes(&make_retro_wav(&[(400.0, 0.09), (260.0, 0.10), (150.0, 0.18)])).await.unwrap();

    let save_path = if cfg!(target_os = "android") { "/data/data/com.miroslav.retrosnake/files/best.txt" } else { "retro_snake_best.txt" };
    let mut best: u32 = fs::read_to_string(save_path).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    let mut state = State::Menu;
    let mut score: u32 = 0;
    let mut rec_beaten = false;
    let mut snake = vec![(10, 9), (9, 9), (8, 9)];
    let mut dir = Dir::Right;
    let mut next_dir = Dir::Right;
    let mut apple = spawn_apple(&snake);
    let mut timer = 0.0;
    let mut die_step = 0usize;

    loop {
        // Определяем реальное разрешение экрана телефона / окна
        let sw = screen_width();
        let sh = screen_height();
        clear_background(NOKIA_BG);

        // Адаптивный расчет пропорций под любой экран
        let hud_h = (sh * 0.075).clamp(44.0, 80.0);
        let controls_h = sh * 0.31;
        let avail_w = sw * 0.94;
        let avail_h = (sh - hud_h - controls_h - 20.0).max(120.0);

        let cell = (avail_w / GRID_W as f32).min(avail_h / GRID_H as f32);
        let field_w = cell * GRID_W as f32;
        let field_h = cell * GRID_H as f32;
        let field_x = (sw - field_w) / 2.0;
        let field_y = hud_h + 10.0;

        // Верхняя панель (HUD)
        draw_line(0.0, hud_h, sw, hud_h, 3.0, NOKIA_DARK);
        let hud_font = (hud_h * 0.52).clamp(20.0, 42.0);
        draw_text(&format!("SCORE: {:03}", score), field_x, hud_h * 0.68, hud_font, NOKIA_DARK);

        let cup_s = hud_font * 0.75;
        let best_txt = format!("{:03}", best);
        let best_dim = measure_text(&best_txt, None, hud_font as u16, 1.0);
        let cx = field_x + field_w - best_dim.width - cup_s - 10.0;
        let cy = hud_h * 0.25;
        draw_rectangle(cx + cup_s * 0.15, cy, cup_s * 0.7, cup_s * 0.45, GOLD_CUP);
        draw_rectangle(cx + cup_s * 0.25, cy + cup_s * 0.45, cup_s * 0.5, cup_s * 0.2, GOLD_CUP);
        draw_rectangle(cx + cup_s * 0.4, cy + cup_s * 0.65, cup_s * 0.2, cup_s * 0.2, GOLD_CUP);
        draw_rectangle(cx + cup_s * 0.2, cy + cup_s * 0.85, cup_s * 0.6, cup_s * 0.15, NOKIA_DARK);
        draw_text(&best_txt, field_x + field_w - best_dim.width, hud_h * 0.68, hud_font, NOKIA_DARK);

        // Рамка и сетка игрового поля
        draw_rectangle_lines(field_x - 3.0, field_y - 3.0, field_w + 6.0, field_h + 6.0, 3.0, NOKIA_DARK);
        for x in 0..GRID_W {
            for y in 0..GRID_H {
                draw_rectangle_lines(field_x + x as f32 * cell, field_y + y as f32 * cell, cell, cell, 1.0, NOKIA_GRID);
            }
        }

        // 4 сенсорные кнопки под полем (работают и курсором мыши на ПК, и пальцем на Android)
        let (ptr, down, pressed) = get_pointer();
        let pad_top = field_y + field_h + 16.0;
        let pad_avail_h = (sh - pad_top - 14.0).max(90.0);
        let btn_h = (pad_avail_h - 12.0) / 2.0;
        let btn_w = ((sw * 0.90 - 24.0) / 3.0).min(btn_h * 1.45);
        let center_x = sw / 2.0 - btn_w / 2.0;

        let up_btn = Rect::new(center_x, pad_top, btn_w, btn_h);
        let left_btn = Rect::new(center_x - btn_w - 12.0, pad_top + btn_h + 12.0, btn_w, btn_h);
        let down_btn = Rect::new(center_x, pad_top + btn_h + 12.0, btn_w, btn_h);
        let right_btn = Rect::new(center_x + btn_w + 12.0, pad_top + btn_h + 12.0, btn_w, btn_h);

        let go_up = draw_ctrl_btn(up_btn, Dir::Up, ptr, down, pressed) || is_key_pressed(KeyCode::W);
        let go_left = draw_ctrl_btn(left_btn, Dir::Left, ptr, down, pressed) || is_key_pressed(KeyCode::A);
        let go_down = draw_ctrl_btn(down_btn, Dir::Down, ptr, down, pressed) || is_key_pressed(KeyCode::S);
        let go_right = draw_ctrl_btn(right_btn, Dir::Right, ptr, down, pressed) || is_key_pressed(KeyCode::D);

        let dt = get_frame_time();
        match state {
            State::Menu | State::Over => {
                let title = if state == State::Menu { "RETRO SNAKE" } else { "GAME OVER" };
                let title_sz = (field_w * 0.11).clamp(24.0, 56.0) as u16;
                let td = measure_text(title, None, title_sz, 1.0);
                draw_text(title, (sw - td.width) / 2.0, field_y + field_h * 0.30, title_sz as f32, NOKIA_DARK);

                let m_w = field_w * 0.52;
                let m_h = (field_h * 0.14).clamp(38.0, 64.0);
                let play_r = Rect::new((sw - m_w) / 2.0, field_y + field_h * 0.42, m_w, m_h);
                let exit_r = Rect::new((sw - m_w) / 2.0, field_y + field_h * 0.42 + m_h + 14.0, m_w, m_h);

                if draw_menu_btn(play_r, "PLAY", ptr, down, pressed) || is_key_pressed(KeyCode::Enter) {
                    snake = vec![(10, 9), (9, 9), (8, 9)];
                    dir = Dir::Right;
                    next_dir = Dir::Right;
                    score = 0;
                    rec_beaten = false;
                    apple = spawn_apple(&snake);
                    state = State::Play;
                    timer = 0.0;
                }
                if draw_menu_btn(exit_r, "EXIT", ptr, down, pressed) || is_key_pressed(KeyCode::Escape) {
                    break;
                }
            }
            State::Play => {
                if go_up && dir != Dir::Down { next_dir = Dir::Up; }
                else if go_down && dir != Dir::Up { next_dir = Dir::Down; }
                else if go_left && dir != Dir::Right { next_dir = Dir::Left; }
                else if go_right && dir != Dir::Left { next_dir = Dir::Right; }

                let step = (0.14 - score as f32 * 0.002).max(0.055);
                timer += dt;
                if timer >= step {
                    timer = 0.0;
                    dir = next_dir;
                    let (hx, hy) = *snake.first().unwrap();
                    let nh = match dir {
                        Dir::Up => (hx, hy - 1),
                        Dir::Down => (hx, hy + 1),
                        Dir::Left => (hx - 1, hy),
                        Dir::Right => (hx + 1, hy),
                    };
                    if nh.0 < 0 || nh.0 >= GRID_W || nh.1 < 0 || nh.1 >= GRID_H || snake.contains(&nh) {
                        play_sound_once(&sfx_die);
                        state = State::Dying;
                        die_step = 0;
                    } else {
                        snake.insert(0, nh);
                        if nh == apple {
                            score += 1;
                            if score > best {
                                best = score;
                                let _ = fs::write(save_path, best.to_string());
                                if !rec_beaten { rec_beaten = true; play_sound_once(&sfx_rec); }
                                else { play_sound_once(&sfx_eat); }
                            } else { play_sound_once(&sfx_eat); }
                            apple = spawn_apple(&snake);
                        } else { snake.pop(); }
                    }
                }
            }
            State::Dying => {
                timer += dt;
                if timer >= 0.055 {
                    timer = 0.0;
                    die_step += 1;
                    if snake.len() > 1 {
                        snake.pop();
                        for (i, seg) in snake.iter_mut().enumerate().skip(1) {
                            let shift = if (i + die_step) % 2 == 0 { 1 } else { -1 };
                            seg.0 = (seg.0 + shift).clamp(0, GRID_W - 1);
                        }
                    } else {
                        snake.clear();
                        state = State::Over;
                    }
                }
            }
        }

        if state == State::Play || state == State::Dying {
            if state == State::Play {
                let ax = field_x + apple.0 as f32 * cell + cell / 2.0;
                let ay = field_y + apple.1 as f32 * cell + cell / 2.0;
                draw_circle(ax, ay + 1.0, cell * 0.42, APPLE_RED);
                draw_circle(ax - cell * 0.12, ay - cell * 0.12, cell * 0.12, WHITE);
            }
            for (i, &(sx, sy)) in snake.iter().enumerate() {
                let px = field_x + sx as f32 * cell + 1.0;
                let py = field_y + sy as f32 * cell + 1.0;
                let sz = cell - 2.0;
                let col = if state == State::Dying { SNAKE_DYING } else if i == 0 { SNAKE_HEAD } else { SNAKE_BODY };
                draw_rectangle(px, py, sz, sz, col);
                draw_rectangle_lines(px, py, sz, sz, 1.5, NOKIA_DARK);
                if i == 0 && state == State::Play {
                    let eye = (sz * 0.18).max(2.0);
                    draw_rectangle(px + sz * 0.2, py + sz * 0.22, eye, eye, NOKIA_DARK);
                    draw_rectangle(px + sz * 0.8 - eye, py + sz * 0.22, eye, eye, NOKIA_DARK);
                }
            }
        }

        next_frame().await;
    }
}
