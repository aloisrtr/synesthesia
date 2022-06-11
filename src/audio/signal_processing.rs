use rustfft::FftPlanner;
use rustfft::num_complex::Complex;

pub fn fft(data: &[f32]) -> Vec<Complex<f32>> {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(data.len());

    let mut result: Vec<Complex<f32>> = data.iter().map(Complex::from).collect();
    fft.process(&mut result);
    result
}