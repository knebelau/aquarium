use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

const TAU: f64 = std::f64::consts::TAU;

// ---------- simple PRNG ----------
struct Rng(u32);
impl Rng {
    fn new(seed: u32) -> Self { Rng(seed.max(1)) }
    fn next(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x as f64) / (u32::MAX as f64)
    }
    fn range(&mut self, a: f64, b: f64) -> f64 { a + (b - a) * self.next() }
    fn int(&mut self, a: i32, b: i32) -> i32 { a + (self.next() * (b - a) as f64) as i32 }
}

// ---------- types ----------
struct Fish {
    x: f64, y: f64,
    vx: f64, vy: f64,
    heading: f64,
    size: f64,
    hue: f64,
    sat: f64,
    light: f64,
    speed: f64,
    phase: f64,
    wiggle: f64,
    target_x: f64, target_y: f64,
    retarget_in: f64,
    fed: f64,
    depth_bias: f64,
}

struct Bubble { x: f64, y: f64, r: f64, vy: f64, wobble: f64, phase: f64 }
struct Pellet { x: f64, y: f64, vx: f64, vy: f64, r: f64, life: f64 }
struct Ripple { x: f64, y: f64, r: f64, life: f64 }
struct Seaweed { x: f64, h: f64, phase: f64, speed: f64, width: f64, hue: f64 }
struct Ray { x: f64, w: f64, speed: f64, phase: f64, alpha: f64 }

fn sand_height(h: f64) -> f64 { h * 0.10 }
fn surface_y(h: f64) -> f64 { h * 0.09 }

struct App {
    w: f64, h: f64,
    t: f64,
    demo: bool,
    demo_done: bool,
    rng: Rng,
    fish: Vec<Fish>,
    bubbles: Vec<Bubble>,
    pellets: Vec<Pellet>,
    ripples: Vec<Ripple>,
    seaweed: Vec<Seaweed>,
    rays: Vec<Ray>,
    bubble_timer: f64,
    vents: Vec<f64>,
}

impl App {
    fn new(w: f64, h: f64) -> Self {
        let mut rng = Rng::new(0x9E3779B9);
        let mut fish = Vec::new();
        let hues = [18.0, 38.0, 205.0, 285.0, 350.0, 160.0, 60.0, 225.0];
        for i in 0..8 {
            fish.push(Fish {
                x: rng.range(w * 0.15, w * 0.85),
                y: rng.range(h * 0.25, h * 0.75),
                vx: 0.0, vy: 0.0,
                heading: rng.range(0.0, TAU),
                size: rng.range(20.0, 34.0),
                hue: hues[i % hues.len()],
                sat: rng.range(65.0, 90.0),
                light: rng.range(55.0, 68.0),
                speed: rng.range(40.0, 75.0),
                phase: rng.range(0.0, TAU),
                wiggle: rng.range(5.0, 8.0),
                target_x: w * 0.5, target_y: h * 0.5,
                retarget_in: 0.0,
                fed: 0.0,
                depth_bias: rng.range(0.25, 0.75),
            });
        }
        let mut seaweed = Vec::new();
        for _ in 0..9 {
            seaweed.push(Seaweed {
                x: rng.range(w * 0.04, w * 0.96),
                h: rng.range(h * 0.12, h * 0.30),
                phase: rng.range(0.0, TAU),
                speed: rng.range(0.8, 1.6),
                width: rng.range(6.0, 13.0),
                hue: rng.range(120.0, 165.0),
            });
        }
        let mut rays = Vec::new();
        for _ in 0..5 {
            rays.push(Ray {
                x: rng.range(0.0, w),
                w: rng.range(50.0, 140.0),
                speed: rng.range(0.15, 0.4),
                phase: rng.range(0.0, TAU),
                alpha: rng.range(0.04, 0.10),
            });
        }
        let vents = vec![w * 0.22, w * 0.55, w * 0.82];
        App {
            w, h, t: 0.0, demo: false, demo_done: false, rng, fish, bubbles: Vec::new(), pellets: Vec::new(),
            ripples: Vec::new(), seaweed, rays, bubble_timer: 0.0, vents,
        }
    }

    fn resize(&mut self, w: f64, h: f64) {
        if self.w < 1.0 || self.h < 1.0 { self.w = w; self.h = h; return; }
        let sx = w / self.w;
        let sy = h / self.h;
        self.w = w; self.h = h;
        for f in self.fish.iter_mut() {
            f.x *= sx; f.y *= sy;
            f.target_x *= sx; f.target_y *= sy;
        }
        for s in self.seaweed.iter_mut() {
            s.x *= sx; s.h *= sy;
        }
        for r in self.rays.iter_mut() { r.x *= sx; }
        self.vents = vec![w * 0.22, w * 0.55, w * 0.82];
    }

    fn scatter_food(&mut self, x: f64, y: f64) {
        let n = self.rng.int(9, 15);
        for _ in 0..n {
            let ang = self.rng.range(0.0, TAU);
            let dist = self.rng.range(4.0, 46.0);
            let px = (x + ang.cos() * dist).clamp(10.0, self.w - 10.0);
            let py = (y + ang.sin() * dist * 0.6).clamp(
                surface_y(self.h) + 20.0,
                self.h - sand_height(self.h) - 10.0,
            );
            self.pellets.push(Pellet {
                x: px, y: py,
                vx: self.rng.range(-12.0, 12.0),
                vy: self.rng.range(4.0, 18.0),
                r: self.rng.range(2.6, 4.2),
                life: 30.0,
            });
        }
        self.ripples.push(Ripple { x, y, r: 6.0, life: 1.0 });
    }

    fn update(&mut self, dt: f64) {
        self.t += dt;
        if self.demo && !self.demo_done && self.t > 1.5 {
            self.demo_done = true;
            self.scatter_food(self.w * 0.5, self.h * 0.55);
        }
        let (w, h) = (self.w, self.h);
        let surface = surface_y(h);
        let sand_top = h - sand_height(h);

        // bubbles from vents
        self.bubble_timer -= dt;
        if self.bubble_timer <= 0.0 {
            self.bubble_timer = self.rng.range(0.28, 0.55);
            let vx = self.vents[self.rng.int(0, self.vents.len() as i32) as usize];
            self.bubbles.push(Bubble {
                x: vx + self.rng.range(-6.0, 6.0),
                y: sand_top + self.rng.range(-4.0, 4.0),
                r: self.rng.range(2.5, 5.5),
                vy: self.rng.range(38.0, 70.0),
                wobble: self.rng.range(14.0, 34.0),
                phase: self.rng.range(0.0, TAU),
            });
        }
        // occasional bubble from a fish
        if self.rng.next() < dt * 0.18 && !self.fish.is_empty() {
            let i = self.rng.int(0, self.fish.len() as i32) as usize;
            let f = &self.fish[i];
            self.bubbles.push(Bubble {
                x: f.x + f.heading.cos() * f.size * 1.2,
                y: f.y + f.heading.sin() * f.size * 1.2,
                r: self.rng.range(1.8, 3.6),
                vy: self.rng.range(30.0, 50.0),
                wobble: self.rng.range(10.0, 22.0),
                phase: self.rng.range(0.0, TAU),
            });
        }

        // update bubbles
        for b in self.bubbles.iter_mut() {
            b.y -= b.vy * dt;
            b.x += b.wobble * dt * (self.t * 2.6 + b.phase).sin();
            b.r += dt * 1.1;
        }
        self.bubbles.retain(|b| b.y > surface + 4.0);

        // update pellets
        for p in self.pellets.iter_mut() {
            p.vy += 26.0 * dt;
            p.vx *= 1.0 - 1.4 * dt;
            p.vy = p.vy.min(46.0);
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            if p.y > sand_top - p.r { p.y = sand_top - p.r; p.vy = 0.0; p.vx = 0.0; }
            p.life -= dt;
        }
        self.pellets.retain(|p| p.life > 0.0);

        // update ripples
        for r in self.ripples.iter_mut() { r.r += 160.0 * dt; r.life -= dt * 1.4; }
        self.ripples.retain(|r| r.life > 0.0);

        // fish
        let has_food = !self.pellets.is_empty();
        for i in 0..self.fish.len() {
            let target = if has_food {
                let mut best: Option<(usize, f64)> = None;
                for (j, p) in self.pellets.iter().enumerate() {
                    let d = (p.x - self.fish[i].x).powi(2) + (p.y - self.fish[i].y).powi(2);
                    if best.map_or(true, |(_, bd)| d < bd) { best = Some((j, d)); }
                }
                let (j, _) = best.unwrap();
                let p = &self.pellets[j];
                (p.x, p.y)
            } else {
                let f = &mut self.fish[i];
                f.retarget_in -= dt;
                if f.retarget_in <= 0.0 {
                    f.retarget_in = self.rng.range(2.5, 6.0);
                    f.target_x = self.rng.range(w * 0.08, w * 0.92);
                    f.target_y = h * f.depth_bias + self.rng.range(-h * 0.12, h * 0.12);
                    if self.rng.next() < 0.18 {
                        f.depth_bias = self.rng.range(0.55, 0.85);
                    } else if self.rng.next() < 0.12 {
                        f.depth_bias = self.rng.range(0.2, 0.4);
                    }
                }
                (self.fish[i].target_x, self.fish[i].target_y)
            };

            let f = &mut self.fish[i];
            f.fed = (f.fed - dt).max(0.0);

            let dx = target.0 - f.x;
            let dy = target.1 - f.y;
            let dist = (dx * dx + dy * dy).sqrt().max(0.001);
            let speed_boost = if f.fed > 0.0 { 1.6 } else { 1.0 };
            let desired_speed = if has_food && dist < 90.0 { f.speed * 1.35 * speed_boost } else { f.speed * speed_boost };
            let desired_vx = dx / dist * desired_speed;
            let desired_vy = dy / dist * desired_speed;

            let margin = 70.0;
            let mut repel_x = 0.0;
            let mut repel_y = 0.0;
            if f.x < margin { repel_x += (margin - f.x) / margin * 1.6; }
            if f.x > w - margin { repel_x -= (f.x - (w - margin)) / margin * 1.6; }
            if f.y < surface + 40.0 { repel_y += (surface + 40.0 - f.y) / 40.0 * 1.8; }
            if f.y > sand_top - 30.0 { repel_y -= (f.y - (sand_top - 30.0)) / 30.0 * 1.8; }

            let steer = 2.4 * dt;
            f.vx += ((desired_vx + repel_x * f.speed) - f.vx) * steer;
            f.vy += ((desired_vy + repel_y * f.speed) - f.vy) * steer;

            f.x += f.vx * dt;
            f.y += f.vy * dt;
            f.x = f.x.clamp(14.0, w - 14.0);
            f.y = f.y.clamp(surface + 16.0, sand_top - 8.0);

            let spd = (f.vx * f.vx + f.vy * f.vy).sqrt();
            if spd > 4.0 {
                let target_heading = f.vy.atan2(f.vx);
                let mut diff = target_heading - f.heading;
                while diff > std::f64::consts::PI { diff -= TAU; }
                while diff < -std::f64::consts::PI { diff += TAU; }
                f.heading += diff * (5.0 * dt).min(1.0);
            }
        }

        // eating
        for i in 0..self.fish.len() {
            let f = &self.fish[i];
            let fx = f.x; let fy = f.y; let fsize = f.size;
            for j in (0..self.pellets.len()).rev() {
                let p = &self.pellets[j];
                let d = (p.x - fx).powi(2) + (p.y - fy).powi(2);
                let eat_r = (fsize * 0.9 + p.r) * (fsize * 0.9 + p.r);
                if d < eat_r {
                    self.pellets.remove(j);
                    self.fish[i].fed = 1.2;
                    for _ in 0..3 {
                        self.bubbles.push(Bubble {
                            x: fx + self.rng.range(-6.0, 6.0),
                            y: fy + self.rng.range(-6.0, 6.0),
                            r: self.rng.range(1.5, 3.2),
                            vy: self.rng.range(35.0, 60.0),
                            wobble: self.rng.range(10.0, 20.0),
                            phase: self.rng.range(0.0, TAU),
                        });
                    }
                    break;
                }
            }
        }
    }

    fn draw(&self, ctx: &CanvasRenderingContext2d) {
        let (w, h) = (self.w, self.h);
        let surface = surface_y(h);
        let sand_top = h - sand_height(h);

        // water background
        let grad = ctx.create_linear_gradient(0.0, 0.0, 0.0, h);
        grad.add_color_stop(0.0, "#2fa3c7").unwrap();
        grad.add_color_stop(0.35, "#1d7fa8").unwrap();
        grad.add_color_stop(0.75, "#0e4f7c").unwrap();
        grad.add_color_stop(1.0, "#082f52").unwrap();
        ctx.set_fill_style_canvas_gradient(&grad);
        ctx.fill_rect(0.0, 0.0, w, h);

        // light rays
        for ray in &self.rays {
            let sway = (self.t * ray.speed + ray.phase).sin() * 60.0;
            let rx = ray.x + sway;
            let grad = ctx.create_linear_gradient(rx, 0.0, rx + ray.w * 0.6, h);
            grad.add_color_stop(0.0, &format!("rgba(255,255,230,{})", ray.alpha)).unwrap();
            grad.add_color_stop(1.0, "rgba(255,255,230,0)").unwrap();
            ctx.set_fill_style_canvas_gradient(&grad);
            ctx.begin_path();
            ctx.move_to(rx, 0.0);
            ctx.line_to(rx + ray.w, 0.0);
            ctx.line_to(rx + ray.w * 0.35 + 90.0, h);
            ctx.line_to(rx + 90.0, h);
            ctx.close_path();
            ctx.fill();
        }

        // seaweed (behind fish)
        for s in &self.seaweed {
            let sway = (self.t * s.speed + s.phase).sin();
            let base_y = sand_top + 6.0;
            let tip_x = s.x + sway * s.h * 0.28;
            let tip_y = base_y - s.h;
            let mid_x = s.x + sway * s.h * 0.12;
            let mid_y = base_y - s.h * 0.55;
            ctx.begin_path();
            ctx.move_to(s.x - s.width * 0.5, base_y);
            ctx.quadratic_curve_to(mid_x - s.width * 0.5, mid_y, tip_x, tip_y);
            ctx.quadratic_curve_to(mid_x + s.width * 0.5, mid_y, s.x + s.width * 0.5, base_y);
            ctx.close_path();
            let grad = ctx.create_linear_gradient(0.0, base_y, 0.0, tip_y);
            grad.add_color_stop(0.0, &format!("hsl({}, 45%, 22%)", s.hue)).unwrap();
            grad.add_color_stop(1.0, &format!("hsl({}, 55%, 40%)", s.hue)).unwrap();
            ctx.set_fill_style_canvas_gradient(&grad);
            ctx.fill();
        }

        // sand
        ctx.begin_path();
        ctx.move_to(0.0, h);
        ctx.line_to(0.0, sand_top);
        for i in 0..=40 {
            let x = i as f64 / 40.0 * w;
            let y = sand_top + (x * 0.021 + 1.7).sin() * 6.0 + (x * 0.008).cos() * 4.0;
            ctx.line_to(x, y);
        }
        ctx.line_to(w, h);
        ctx.close_path();
        let grad = ctx.create_linear_gradient(0.0, sand_top - 10.0, 0.0, h);
        grad.add_color_stop(0.0, "#e8d5a3").unwrap();
        grad.add_color_stop(0.5, "#d9bd82").unwrap();
        grad.add_color_stop(1.0, "#b99a5e").unwrap();
        ctx.set_fill_style_canvas_gradient(&grad);
        ctx.fill();

        // pellets
        for p in &self.pellets {
            ctx.begin_path();
            ctx.arc(p.x, p.y, p.r + 2.5, 0.0, TAU).unwrap();
            ctx.set_fill_style_str("rgba(255, 230, 160, 0.25)");
            ctx.fill();
            ctx.begin_path();
            ctx.arc(p.x, p.y, p.r, 0.0, TAU).unwrap();
            ctx.set_fill_style_str("#ffe9a8");
            ctx.fill();
            ctx.begin_path();
            ctx.arc(p.x - p.r * 0.3, p.y - p.r * 0.3, p.r * 0.35, 0.0, TAU).unwrap();
            ctx.set_fill_style_str("#fff7dd");
            ctx.fill();
        }

        // bubbles
        for b in &self.bubbles {
            ctx.begin_path();
            ctx.arc(b.x, b.y, b.r, 0.0, TAU).unwrap();
            ctx.set_fill_style_str("rgba(200, 235, 255, 0.14)");
            ctx.fill();
            ctx.set_stroke_style_str("rgba(230, 245, 255, 0.55)");
            ctx.set_line_width(1.2);
            ctx.stroke();
            ctx.begin_path();
            ctx.arc(b.x - b.r * 0.35, b.y - b.r * 0.35, b.r * 0.28, 0.0, TAU).unwrap();
            ctx.set_fill_style_str("rgba(255,255,255,0.75)");
            ctx.fill();
        }

        // fish
        for f in &self.fish {
            draw_fish(ctx, f, self.t);
        }

        // surface wave
        ctx.begin_path();
        for i in 0..=60 {
            let x = i as f64 / 60.0 * w;
            let y = surface + (x * 0.02 + self.t * 1.8).sin() * 5.0 + (x * 0.008 - self.t * 1.1).cos() * 3.0;
            if i == 0 { ctx.move_to(x, y); } else { ctx.line_to(x, y); }
        }
        ctx.set_stroke_style_str("rgba(240, 252, 255, 0.85)");
        ctx.set_line_width(2.5);
        ctx.stroke();
        ctx.begin_path();
        ctx.move_to(0.0, 0.0);
        ctx.line_to(0.0, surface);
        for i in 0..=60 {
            let x = i as f64 / 60.0 * w;
            let y = surface + (x * 0.02 + self.t * 1.8).sin() * 5.0 + (x * 0.008 - self.t * 1.1).cos() * 3.0;
            ctx.line_to(x, y);
        }
        ctx.line_to(w, 0.0);
        ctx.close_path();
        let grad = ctx.create_linear_gradient(0.0, 0.0, 0.0, surface + 12.0);
        grad.add_color_stop(0.0, "rgba(235, 250, 255, 0.95)").unwrap();
        grad.add_color_stop(0.55, "rgba(190, 230, 245, 0.55)").unwrap();
        grad.add_color_stop(1.0, "rgba(160, 215, 240, 0.0)").unwrap();
        ctx.set_fill_style_canvas_gradient(&grad);
        ctx.fill();

        // ripples
        for r in &self.ripples {
            ctx.begin_path();
            ctx.arc(r.x, r.y, r.r, 0.0, TAU).unwrap();
            ctx.set_stroke_style_str(&format!("rgba(255,255,255,{})", 0.6 * r.life));
            ctx.set_line_width(2.0);
            ctx.stroke();
        }

        // vignette
        let grad = ctx.create_radial_gradient(w / 2.0, h / 2.0, h * 0.35, w / 2.0, h / 2.0, h * 0.95).unwrap();
        grad.add_color_stop(0.0, "rgba(0,0,0,0)").unwrap();
        grad.add_color_stop(1.0, "rgba(2, 18, 38, 0.38)").unwrap();
        ctx.set_fill_style_canvas_gradient(&grad);
        ctx.fill_rect(0.0, 0.0, w, h);
    }
}

fn draw_fish(ctx: &CanvasRenderingContext2d, f: &Fish, t: f64) {
    let s = f.size;
    let wag = (t * f.wiggle + f.phase).sin() * 0.55;

    ctx.save();
    ctx.translate(f.x, f.y).unwrap();
    ctx.rotate(f.heading).unwrap();

    // tail
    ctx.begin_path();
    ctx.move_to(-s * 0.82, 0.0);
    ctx.quadratic_curve_to(-s * 1.25, wag * s * 0.55, -s * 1.65, wag * s * 0.55 - s * 0.52);
    ctx.line_to(-s * 1.65, wag * s * 0.55 + s * 0.52);
    ctx.quadratic_curve_to(-s * 1.25, wag * s * 0.55, -s * 0.82, 0.0);
    ctx.close_path();
    ctx.set_fill_style_str(&format!("hsl({}, {}%, {}%)", f.hue, f.sat, (f.light - 10.0).max(15.0)));
    ctx.fill();

    // dorsal fin
    ctx.begin_path();
    ctx.move_to(-s * 0.35, -s * 0.42);
    ctx.quadratic_curve_to(s * 0.05, -s * 1.05, s * 0.42, -s * 0.38);
    ctx.close_path();
    ctx.set_fill_style_str(&format!("hsl({}, {}%, {}%)", f.hue, f.sat, (f.light - 6.0).max(15.0)));
    ctx.fill();

    // pectoral fin
    ctx.begin_path();
    ctx.move_to(s * 0.1, s * 0.18);
    ctx.quadratic_curve_to(-s * 0.05 + wag * s * 0.12, s * 0.85, -s * 0.42, s * 0.55);
    ctx.quadratic_curve_to(-s * 0.15, s * 0.28, s * 0.1, s * 0.18);
    ctx.close_path();
    ctx.set_fill_style_str(&format!("hsl({}, {}%, {}%)", f.hue, f.sat * 0.9, (f.light + 12.0).min(88.0)));
    ctx.fill();

    // body
    let grad = ctx.create_linear_gradient(0.0, -s * 0.6, 0.0, s * 0.6);
    grad.add_color_stop(0.0, &format!("hsl({}, {}%, {}%)", f.hue, f.sat, (f.light + 12.0).min(85.0))).unwrap();
    grad.add_color_stop(0.55, &format!("hsl({}, {}%, {}%)", f.hue, f.sat, f.light)).unwrap();
    grad.add_color_stop(1.0, &format!("hsl({}, {}%, {}%)", f.hue, f.sat * 0.85, (f.light - 22.0).max(18.0))).unwrap();
    ctx.begin_path();
    ctx.ellipse(0.0, 0.0, s, s * 0.56, 0.0, 0.0, TAU).unwrap();
    ctx.set_fill_style_canvas_gradient(&grad);
    ctx.fill();

    // stripes (clipped to body)
    ctx.save();
    ctx.begin_path();
    ctx.ellipse(0.0, 0.0, s, s * 0.56, 0.0, 0.0, TAU).unwrap();
    ctx.clip();
    ctx.set_fill_style_str(&format!("hsla({}, {}%, 22%, 0.45)", f.hue, f.sat));
    for k in 0..3 {
        let sx = -s * 0.45 + k as f64 * s * 0.5;
        ctx.begin_path();
        ctx.ellipse(sx, 0.0, s * 0.12, s * 0.6, 0.0, 0.0, TAU).unwrap();
        ctx.fill();
    }
    ctx.restore();

    // belly shine
    ctx.begin_path();
    ctx.ellipse(s * 0.1, s * 0.2, s * 0.62, s * 0.22, 0.0, 0.0, TAU).unwrap();
    ctx.set_fill_style_str("rgba(255,255,255,0.18)");
    ctx.fill();

    // eye
    ctx.begin_path();
    ctx.arc(s * 0.58, -s * 0.16, s * 0.17, 0.0, TAU).unwrap();
    ctx.set_fill_style_str("#ffffff");
    ctx.fill();
    ctx.begin_path();
    ctx.arc(s * 0.63, -s * 0.16, s * 0.09, 0.0, TAU).unwrap();
    ctx.set_fill_style_str("#101418");
    ctx.fill();
    ctx.begin_path();
    ctx.arc(s * 0.66, -s * 0.20, s * 0.035, 0.0, TAU).unwrap();
    ctx.set_fill_style_str("#ffffff");
    ctx.fill();

    // mouth shine when fed
    if f.fed > 0.0 {
        ctx.begin_path();
        ctx.arc(s * 0.86, s * 0.02, s * 0.10, 0.0, TAU).unwrap();
        ctx.set_fill_style_str(&format!("rgba(255, 240, 180, {})", 0.5 * f.fed));
        ctx.fill();
    }

    ctx.restore();
}

// ---------- bootstrap ----------
thread_local! {
    static DEBUG: std::cell::Cell<bool> = std::cell::Cell::new(false);
    static FRAMES: std::cell::Cell<u32> = std::cell::Cell::new(0);
    static DEMO_DONE: std::cell::Cell<bool> = std::cell::Cell::new(false);
    static PENDING_CLICKS: std::cell::RefCell<Vec<(f64, f64)>> = std::cell::RefCell::new(Vec::new());
    static LAST_TS: std::cell::RefCell<f64> = std::cell::RefCell::new(0.0);
    static RAF_CLOSURE: std::cell::RefCell<Option<Closure<dyn FnMut(f64)>>> = std::cell::RefCell::new(None);
}

fn demo_mode() -> bool {
    let Some(win) = web_sys::window() else { return false };
    let Some(doc) = win.document() else { return false };
    let url = doc.url().unwrap_or_default();
    url.contains("demo=1")
}

fn fast_forward_seconds() -> f64 {
    let Some(win) = web_sys::window() else { return 0.0 };
    let Some(doc) = win.document() else { return 0.0 };
    let url = doc.url().unwrap_or_default();
    let mut ff = 0.0;
    if let Some(q) = url.split('?').nth(1) {
        for part in q.split('&') {
            if let Some(v) = part.strip_prefix("ff=") {
                ff = v.parse().unwrap_or(0.0);
            }
        }
    }
    ff
}

fn debug_mode() -> bool {
    let Some(win) = web_sys::window() else { return false };
    let Some(doc) = win.document() else { return false };
    let url = doc.url().unwrap_or_default();
    url.contains("debug=1")
}

#[wasm_bindgen(start)]
pub fn main() {
    let window = web_sys::window().expect("no window");
    let document = window.document().expect("no document");
    let canvas: HtmlCanvasElement = document
        .get_element_by_id("aquarium")
        .expect("canvas missing")
        .dyn_into()
        .expect("not a canvas");
    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .expect("get_context failed")
        .expect("no 2d context")
        .dyn_into()
        .expect("not a 2d context");

    let dpr = window.device_pixel_ratio().max(1.0);
    let css_w = window.inner_width().unwrap().as_f64().unwrap() as f64;
    let css_h = window.inner_height().unwrap().as_f64().unwrap() as f64;
    canvas.set_width((css_w * dpr) as u32);
    canvas.set_height((css_h * dpr) as u32);
    canvas.set_attribute("style", &format!("width:{}px;height:{}px;", css_w, css_h)).unwrap();
    ctx.scale(dpr as f64, dpr as f64).unwrap();

    DEBUG.with(|d| d.set(debug_mode()));
    let mut app = App::new(css_w, css_h);
    app.demo = demo_mode();
    let ff = fast_forward_seconds();
    if ff > 0.0 {
        let steps = (ff * 60.0) as u32;
        for _ in 0..steps {
            app.update(1.0 / 60.0);
        }
    }

    if DEBUG.with(|d| d.get()) {
        let doc = window.document().unwrap();
        let el = doc.create_element("div").unwrap();
        el.set_attribute("id", "debug").unwrap();
        el.set_attribute("style", "position:fixed;bottom:8px;left:8px;color:#fff;background:rgba(0,0,0,0.55);padding:6px 10px;border-radius:8px;font:12px monospace;z-index:99;pointer-events:none;").unwrap();
        doc.body().unwrap().append_child(&el).unwrap();
    }

    // click handler
    let click_closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
        let x = e.client_x() as f64;
        let y = e.client_y() as f64;
        PENDING_CLICKS.with(|c| c.borrow_mut().push((x, y)));
    });
    let cb: &js_sys::Function = click_closure.as_js_value().unchecked_ref();
    canvas.add_event_listener_with_callback("pointerdown", cb).unwrap();
    click_closure.forget();

    // main loop
    let loop_closure = Closure::<dyn FnMut(f64)>::new(move |timestamp: f64| {
        let win = web_sys::window().unwrap();
        let w = win.inner_width().unwrap().as_f64().unwrap() as f64;
        let h = win.inner_height().unwrap().as_f64().unwrap() as f64;
        if (app.w - w).abs() > 1.0 || (app.h - h).abs() > 1.0 {
            let dpr = win.device_pixel_ratio().max(1.0);
            canvas.set_width((w * dpr) as u32);
            canvas.set_height((h * dpr) as u32);
            canvas.set_attribute("style", &format!("width:{}px;height:{}px;", w, h)).unwrap();
            ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0).unwrap();
            app.resize(w, h);
        }

        PENDING_CLICKS.with(|c| {
            for (x, y) in c.borrow_mut().drain(..) {
                app.scatter_food(x, y);
            }
        });

        let dt = (timestamp - LAST_TS.with(|t| *t.borrow())).max(0.0) as f64 / 1000.0;
        LAST_TS.with(|t| *t.borrow_mut() = timestamp);
        let dt = dt.min(0.05);

        app.update(dt);
        app.draw(&ctx);

        if DEBUG.with(|d| d.get()) {
            let frames = FRAMES.with(|f| { let v = f.get() + 1; f.set(v); v });
            let win2 = web_sys::window().unwrap();
            let doc = win2.document().unwrap();
            if let Some(el) = doc.get_element_by_id("debug") {
                el.set_text_content(Some(&format!(
                    "frames={} t={:.2}s dt={:.3} fish={} bubbles={} pellets={} ripples={}",
                    frames, app.t, dt, app.fish.len(), app.bubbles.len(), app.pellets.len(), app.ripples.len()
                )));
            }
        }

        RAF_CLOSURE.with(|c| {
            if let Some(cl) = c.borrow().as_ref() {
                let cb: &js_sys::Function = cl.as_js_value().unchecked_ref();
                let _ = win.request_animation_frame(cb);
            }
        });
    });

    LAST_TS.with(|t| *t.borrow_mut() = 0.0);
    RAF_CLOSURE.with(|c| *c.borrow_mut() = Some(loop_closure));

    RAF_CLOSURE.with(|c| {
        if let Some(cl) = c.borrow().as_ref() {
            let cb: &js_sys::Function = cl.as_js_value().unchecked_ref();
            let _ = window.request_animation_frame(cb);
        }
    });
}
