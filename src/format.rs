//! Shared human-readable formatting helpers for HotWire instrument UIs.
//!
//! These functions intentionally keep the decimal precision used by the
//! original per-app formatters so migrating call sites does not change the
//! rendered values.

/// Formats a non-negative duration using ns / µs / ms / s.
pub fn format_duration(seconds: f64) -> String {
    if !seconds.is_finite() || seconds <= 0.0 {
        return "0 s".to_string();
    }
    let abs = seconds.abs();
    if abs < 1e-6 {
        let ns = seconds * 1e9;
        if abs < 1e-7 {
            format!("{ns:.1} ns")
        } else {
            format!("{ns:.0} ns")
        }
    } else if abs < 1e-3 {
        let us = seconds * 1e6;
        if abs < 1e-5 {
            format!("{us:.2} µs")
        } else {
            format!("{us:.1} µs")
        }
    } else if abs < 1.0 {
        let ms = seconds * 1e3;
        if abs < 0.01 {
            format!("{ms:.2} ms")
        } else {
            format!("{ms:.1} ms")
        }
    } else {
        format!("{seconds:.2} s")
    }
}

/// Formats a signed duration with a leading minus sign when negative.
pub fn format_signed_duration(seconds: f64) -> String {
    if seconds < 0.0 {
        format!("-{}", format_duration(seconds.abs()))
    } else {
        format_duration(seconds)
    }
}

/// Formats the sample period for a sample rate in Hz.
pub fn format_resolution(rate_hz: u32) -> String {
    if rate_hz == 0 {
        return "-".to_string();
    }
    let period_sec = 1.0 / (rate_hz as f64);
    if period_sec < 1e-6 {
        format!("{} ns", (period_sec * 1e9).round() as u64)
    } else if period_sec < 1e-3 {
        let us = period_sec * 1e6;
        if us >= 10.0 {
            format!("{} µs", us.round() as u64)
        } else {
            format!("{us:.1} µs")
        }
    } else {
        format!("{:.1} ms", period_sec * 1e3)
    }
}

/// Formats a frequency with 2 decimals in kHz/MHz (Logic Analyzer style).
pub fn format_frequency(hz: f64) -> String {
    if !hz.is_finite() || hz <= 0.0 {
        return "-".to_string();
    }
    if hz >= 1e6 {
        format!("{:.2} MHz", hz / 1e6)
    } else if hz >= 1e3 {
        format!("{:.2} kHz", hz / 1e3)
    } else {
        format!("{hz:.1} Hz")
    }
}

/// Formats a frequency with 3 decimals in MHz (Scope style).
pub fn format_frequency_precise(hz: f32) -> String {
    if hz >= 1_000_000.0 {
        format!("{:.3} MHz", hz / 1_000_000.0)
    } else if hz >= 1_000.0 {
        format!("{:.2} kHz", hz / 1_000.0)
    } else {
        format!("{:.1} Hz", hz)
    }
}

/// Formats a SigGen frequency, keeping extra precision below 10 Hz.
pub fn format_siggen_frequency(hz: f64) -> String {
    if hz >= 1_000_000.0 {
        format!("{:.3} MHz", hz / 1_000_000.0)
    } else if hz >= 1_000.0 {
        format!("{:.2} kHz", hz / 1_000.0)
    } else if hz >= 10.0 {
        format!("{:.1} Hz", hz)
    } else {
        format!("{:.2} Hz", hz)
    }
}

/// Formats a time in microseconds (Scope measurement style).
pub fn format_time_us(us: f32) -> String {
    let abs = us.abs();
    if abs >= 1_000_000.0 {
        format!("{:.3} s", us / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("{:.2} ms", us / 1_000.0)
    } else if abs >= 1.0 {
        format!("{:.1} µs", us)
    } else {
        format!("{:.0} ns", us * 1_000.0)
    }
}

/// Formats a millivolt value as mV below 1 V and V above.
pub fn format_voltage_mv(mv: f32) -> String {
    if mv.abs() >= 1000.0 {
        format!("{:.3} V", mv / 1000.0)
    } else {
        format!("{:.1} mV", mv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_durations_resolutions_and_frequencies() {
        assert_eq!(format_duration(50e-9), "50.0 ns");
        assert_eq!(format_duration(16.384e-3), "16.4 ms");
        assert_eq!(format_duration(16.384), "16.38 s");
        assert_eq!(format_signed_duration(-100e-6), "-100.0 µs");
        assert_eq!(format_resolution(20_000_000), "50 ns");
        assert_eq!(format_resolution(1_000_000), "1.0 µs");
        assert_eq!(format_resolution(1_000), "1.0 ms");
        assert_eq!(format_frequency(5_000_000.0), "5.00 MHz");
        assert_eq!(format_frequency(25_000.0), "25.00 kHz");
    }

    #[test]
    fn formats_scope_and_siggen_values() {
        assert_eq!(format_frequency_precise(1_500_000.0), "1.500 MHz");
        assert_eq!(format_frequency_precise(25_000.0), "25.00 kHz");
        assert_eq!(format_siggen_frequency(5.0), "5.00 Hz");
        assert_eq!(format_siggen_frequency(50_000.0), "50.00 kHz");
        assert_eq!(format_time_us(1_500.0), "1.50 ms");
        assert_eq!(format_time_us(0.5), "500 ns");
        assert_eq!(format_voltage_mv(1500.0), "1.500 V");
        assert_eq!(format_voltage_mv(250.0), "250.0 mV");
    }
}
