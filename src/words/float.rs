//! The FLOATING word set and its extensions. Floats are IEEE 754 doubles
//! on a stack of their own; FCONSTANT FVARIABLE and the field words are in
//! the prelude.

use crate::*;

pub(crate) fn install(f: &mut Forth) {
    // conversion
    f.prim(">FLOAT", |f| {
        let (a, n) = f.pop2()?;
        let i = f.check(a, n.max(0))?;
        match to_float(&f.mem[i..i + n.max(0) as usize], false) {
            Some(r) => {
                f.fpush(r)?;
                f.push(-1);
            }
            None => f.push(0),
        }
        Ok(())
    });
    f.prim("D>F", |f| {
        let d = f.pop_d()?;
        f.fpush(d as f64)
    });
    f.prim("F>D", |f| {
        let r = f.fpop()?.trunc();
        if !(r >= i128::MIN as f64 && r < i128::MAX as f64) {
            return Err(Throw::new(-11));
        }
        f.push_d(r as i128);
        Ok(())
    });
    f.prim("S>F", |f| {
        let n = f.pop()?;
        f.fpush(n as f64)
    });
    f.prim("F>S", |f| {
        let r = f.fpop()?.trunc();
        if !(r >= Cell::MIN as f64 && r < Cell::MAX as f64) {
            return Err(Throw::new(-11));
        }
        f.push(r as Cell);
        Ok(())
    });

    // memory
    f.prim("F@", |f| {
        let a = f.pop()?;
        let i = f.check(a, FLOAT as Cell)?;
        f.fpush(f.fget(i))
    });
    f.prim("F!", |f| {
        let a = f.pop()?;
        let r = f.fpop()?;
        let i = f.check(a, FLOAT as Cell)?;
        f.fset(i, r);
        Ok(())
    });
    f.prim("SF@", |f| {
        let a = f.pop()?;
        let i = f.check(a, SFLOAT as Cell)?;
        let r = f32::from_le_bytes(f.mem[i..i + SFLOAT].try_into().unwrap());
        f.fpush(r as f64)
    });
    f.prim("SF!", |f| {
        let a = f.pop()?;
        let r = f.fpop()? as f32;
        let i = f.check(a, SFLOAT as Cell)?;
        f.mem[i..i + SFLOAT].copy_from_slice(&r.to_le_bytes());
        Ok(())
    });
    // the float stack
    f.prim("FDEPTH", |f| {
        f.push(f.fstack.len() as Cell);
        Ok(())
    });
    f.prim("FDROP", |f| f.fpop().map(drop));
    f.prim("FDUP", |f| {
        let r = f.fpop()?;
        f.fpush(r)?;
        f.fpush(r)
    });
    f.prim("FSWAP", |f| {
        let (a, b) = f.fpop2()?;
        f.fpush(b)?;
        f.fpush(a)
    });
    f.prim("FOVER", |f| {
        let (a, b) = f.fpop2()?;
        f.fpush(a)?;
        f.fpush(b)?;
        f.fpush(a)
    });
    f.prim("FROT", |f| {
        let (b, c) = f.fpop2()?;
        let a = f.fpop()?;
        f.fpush(b)?;
        f.fpush(c)?;
        f.fpush(a)
    });
    f.imm("FLITERAL", |f| {
        f.need_def()?;
        let r = f.fpop()?;
        f.code.push(Instr::FLit(r));
        Ok(())
    });
    f.prim("FVALUE", |f| {
        let name = f.name()?;
        let r = f.fpop()?;
        f.align_here();
        let addr = f.reserve(FLOAT)?;
        f.fset(addr, r);
        f.define(&name, Kind::FValue(addr));
        Ok(())
    });

    // arithmetic
    f.prim("F+", |f| f.fbinop(|a, b| a + b));
    f.prim("F-", |f| f.fbinop(|a, b| a - b));
    f.prim("F*", |f| f.fbinop(|a, b| a * b));
    f.prim("F/", |f| f.fbinop(|a, b| a / b));
    f.prim("F**", |f| f.fbinop(f64::powf));
    f.prim("FMAX", |f| f.fbinop(fmax));
    f.prim("FMIN", |f| f.fbinop(fmin));
    f.prim("FATAN2", |f| f.fbinop(f64::atan2));
    f.prim("FNEGATE", |f| f.funop(|r| -r));
    f.prim("FABS", |f| f.funop(f64::abs));
    f.prim("FLOOR", |f| f.funop(f64::floor));
    f.prim("FROUND", |f| f.funop(round_even));
    f.prim("FTRUNC", |f| f.funop(f64::trunc));
    f.prim("FSQRT", |f| f.funop(f64::sqrt));
    f.prim("FEXP", |f| f.funop(f64::exp));
    f.prim("FEXPM1", |f| f.funop(f64::exp_m1));
    f.prim("FLN", |f| f.funop(f64::ln));
    f.prim("FLNP1", |f| f.funop(f64::ln_1p));
    f.prim("FLOG", |f| f.funop(f64::log10));
    f.prim("FALOG", |f| f.funop(|r| 10f64.powf(r)));
    f.prim("FSIN", |f| f.funop(f64::sin));
    f.prim("FCOS", |f| f.funop(f64::cos));
    f.prim("FTAN", |f| f.funop(f64::tan));
    f.prim("FASIN", |f| f.funop(f64::asin));
    f.prim("FACOS", |f| f.funop(f64::acos));
    f.prim("FATAN", |f| f.funop(f64::atan));
    f.prim("FSINH", |f| f.funop(f64::sinh));
    f.prim("FCOSH", |f| f.funop(f64::cosh));
    f.prim("FTANH", |f| f.funop(f64::tanh));
    f.prim("FASINH", |f| f.funop(f64::asinh));
    f.prim("FACOSH", |f| f.funop(f64::acosh));
    f.prim("FATANH", |f| f.funop(f64::atanh));
    f.prim("FSINCOS", |f| {
        let r = f.fpop()?;
        f.fpush(r.sin())?;
        f.fpush(r.cos())
    });

    // comparison
    f.prim("F0<", |f| {
        let r = f.fpop()?;
        f.push(flag(r < 0.0));
        Ok(())
    });
    f.prim("F0=", |f| {
        let r = f.fpop()?;
        f.push(flag(r == 0.0));
        Ok(())
    });
    f.prim("F<", |f| {
        let (a, b) = f.fpop2()?;
        f.push(flag(a < b));
        Ok(())
    });
    // F~ ( F: r1 r2 r3 -- ) ( -- flag ): r3 > 0 is an absolute tolerance,
    // r3 < 0 a relative one, and r3 = 0 asks for identical bits. A NaN as
    // the tolerance is never met.
    f.prim("F~", |f| {
        let (r2, r3) = f.fpop2()?;
        let r1 = f.fpop()?;
        let near = if r3.is_nan() {
            false
        } else if r3 > 0.0 {
            (r1 - r2).abs() < r3
        } else if r3 < 0.0 {
            (r1 - r2).abs() < -r3 * (r1.abs() + r2.abs())
        } else {
            r1.to_bits() == r2.to_bits()
        };
        f.push(flag(near));
        Ok(())
    });

    // output
    f.prim("REPRESENT", |f| {
        let (a, u) = f.pop2()?;
        let r = f.fpop()?;
        let u = u.max(0) as usize;
        let i = f.check(a, u as Cell)?;
        match represent(r, u) {
            Some((digits, n)) => {
                f.mem[i..i + u].copy_from_slice(&digits);
                f.push(n as Cell);
                f.push(flag(r.is_sign_negative()));
                f.push(-1);
            }
            None => {
                let text = if r.is_nan() { b"nan" } else { b"inf" };
                for (k, byte) in f.mem[i..i + u].iter_mut().enumerate() {
                    *byte = *text.get(k).unwrap_or(&b' ');
                }
                f.push(0);
                f.push(flag(r.is_sign_negative()));
                f.push(0);
            }
        }
        Ok(())
    });
    f.prim("F.", |f| {
        let r = f.fpop()?;
        f.emit_str(&fixed(r, f.precision));
        Ok(())
    });
    f.prim("FS.", |f| {
        let r = f.fpop()?;
        f.emit_str(&scientific(r, f.precision, false));
        Ok(())
    });
    f.prim("FE.", |f| {
        let r = f.fpop()?;
        f.emit_str(&scientific(r, f.precision, true));
        Ok(())
    });
    f.prim("PRECISION", |f| {
        f.push(f.precision as Cell);
        Ok(())
    });
    f.prim("SET-PRECISION", |f| {
        let u = f.pop()?;
        f.precision = u.clamp(1, MAX_PRECISION as Cell) as usize;
        Ok(())
    });
}

/// FMAX and FMIN: a NaN wins, and +0 is larger than -0.
fn fmax(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        a + b
    } else if a == b {
        if a.is_sign_negative() {
            b
        } else {
            a
        }
    } else {
        a.max(b)
    }
}

fn fmin(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        a + b
    } else if a == b {
        if a.is_sign_negative() {
            a
        } else {
            b
        }
    } else {
        a.min(b)
    }
}
