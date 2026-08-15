pub fn resume_position(position_sec: f64, duration_sec: Option<f64>) -> f64 {
    if !position_sec.is_finite() || position_sec < 5.0 {
        return 0.0;
    }
    if let Some(d) = duration_sec {
        if !d.is_finite() || position_sec > d || position_sec > d - 10.0 {
            return 0.0;
        }
    }
    position_sec
}

#[cfg(test)]
mod tests {
    use super::resume_position;

    #[test]
    fn resume_window() {
        assert_eq!(resume_position(4.0, Some(100.0)), 0.0);
        assert_eq!(resume_position(50.0, Some(100.0)), 50.0);
        assert_eq!(resume_position(95.0, Some(100.0)), 0.0);
        assert_eq!(resume_position(120.0, Some(100.0)), 0.0);
        assert_eq!(resume_position(f64::NAN, Some(100.0)), 0.0);
        assert_eq!(resume_position(12.0, None), 12.0);
        assert_eq!(resume_position(3.0, None), 0.0);
    }
}
