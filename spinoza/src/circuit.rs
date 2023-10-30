//! Abstractions for a quantum circuit
use crate::{
    core::State,
    gates::{apply, c_apply, cc_apply, Gate},
    math::{pow2f, Float, PI},
    measurement::measure_qubit,
};
use std::{collections::HashSet, ops::Index};

/// See <https://en.wikipedia.org/wiki/Quantum_register>
#[derive(Clone)]
pub struct QuantumRegister(pub Vec<usize>);

impl Index<usize> for QuantumRegister {
    type Output = usize;

    #[inline]
    fn index(&self, i: usize) -> &Self::Output {
        &self.0[i]
    }
}

impl QuantumRegister {
    /// Create a new QuantumRegister
    pub fn new(size: usize) -> Self {
        QuantumRegister((0..size).collect())
    }

    /// The length of the quantum register
    #[allow(clippy::len_without_is_empty)]
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Update quantum register by shift
    #[inline]
    pub fn update_shift(&mut self, shift: usize) {
        self.0 = (shift..shift + self.len()).collect();
    }
}

/// Control qubits
#[derive(Clone)]
pub enum Controls {
    /// No controls
    None,
    /// Single Control
    Single(usize),
    /// Multiple Controls
    Ones(Vec<usize>),

    /// Mixed Controls
    #[allow(dead_code)]
    Mixed {
        /// Control qubits
        controls: Vec<usize>,
        /// Zeroes
        zeros: HashSet<usize>,
    },
}

/// QuantumTransformation to be applied to the State
#[derive(Clone)]
pub struct QuantumTransformation {
    /// The quantum logic gate
    pub gate: Gate,
    /// The target qubits
    pub target: usize,
    /// The control qubits
    pub controls: Controls,
}

struct QubitTracker {
    /// Used to keep track of the qubits that were already measured We can use a u64, since the
    /// number of qubits that one can simulate is very small. Hence, 64 bits suffices to keep
    /// track of all qubits.
    measured_qubits: u64,
    /// Used to keep track of the *values* of qubits that were already measured We can use a u64,
    /// since the number of qubits that one can simulate is very small, and the measured values are
    /// either 0 or 1. Hence, 64 bits suffices to keep track of all measurement values.
    measured_qubits_vals: u64,
}

impl QubitTracker {
    pub fn new() -> Self {
        Self {
            measured_qubits: 0,
            measured_qubits_vals: 0,
        }
    }

    fn is_qubit_measured(&self, target_qubit: usize) -> bool {
        ((self.measured_qubits >> target_qubit) & 1) == 1
    }

    #[allow(dead_code)]
    fn get_qubit_measured_val(&mut self, target_qubit: usize) -> u8 {
        ((self.measured_qubits_vals & (1 << target_qubit)) >> target_qubit)
            .try_into()
            .unwrap()
    }

    /// Set the given target qubit as measured
    fn set_measured_qubit(&mut self, target_qubit: usize) {
        self.measured_qubits |= 1 << target_qubit;
    }

    fn set_val_for_measured_qubit(&mut self, target_qubit: usize, value: u8) {
        self.measured_qubits_vals &= !(1 << target_qubit);
        self.measured_qubits_vals |= (value as u64) << target_qubit;
    }
}

/// A model of a Quantum circuit
/// See <https://en.wikipedia.org/wiki/Quantum_circuit>
pub struct QuantumCircuit {
    transformations: Vec<QuantumTransformation>,
    /// The Quantum State to which transformations are applied
    pub state: State,
    qubit_tracker: QubitTracker,
}

impl QuantumCircuit {
    /// Create a new QuantumCircuit from multiple QuantumRegisters
    pub fn new_multi(registers: &mut [&mut QuantumRegister]) -> Self {
        let mut bits = 0;

        for r in registers {
            r.update_shift(bits);
            bits += r.len();
        }
        QuantumCircuit {
            transformations: Vec::new(),
            state: State::new(bits),
            qubit_tracker: QubitTracker::new(),
        }
    }

    /// Create a new QuantumCircuit from a single QuantumRegister
    pub fn new(r: &mut QuantumRegister) -> Self {
        QuantumCircuit::new_multi(&mut [r])
    }

    /// Create a new QuantumCircuit from two QuantumRegisters
    pub fn new2(r0: &mut QuantumRegister, r1: &mut QuantumRegister) -> Self {
        QuantumCircuit::new_multi(&mut [r0, r1])
    }

    /// Get a reference to the Quantum State
    pub fn get_statevector(&self) -> &State {
        &self.state
    }

    /// Measure a single qubit
    #[inline]
    pub fn measure(&mut self, target: usize) {
        self.add(QuantumTransformation {
            gate: Gate::M,
            target,
            controls: Controls::None,
        });
    }

    /// Add the X gate for a given target to the list of QuantumTransformations
    #[inline]
    pub fn x(&mut self, target: usize) {
        self.add(QuantumTransformation {
            gate: Gate::X,
            target,
            controls: Controls::None,
        });
    }

    /// Add the Rx gate for a given target to the list of QuantumTransformations
    #[inline]
    pub fn rx(&mut self, angle: Float, target: usize) {
        self.add(QuantumTransformation {
            gate: Gate::RX(angle),
            target,
            controls: Controls::None,
        });
    }

    /// Add the CX gate for a given target qubit and control qubit to the list of
    /// QuantumTransformations
    #[inline]
    pub fn cx(&mut self, control: usize, target: usize) {
        self.add(QuantumTransformation {
            gate: Gate::X,
            target,
            controls: Controls::Single(control),
        });
    }

    /// Add the CCX gate for a given target qubit and two control qubits to the list of
    /// QuantumTransformations
    #[inline]
    pub fn ccx(&mut self, control1: usize, control2: usize, target: usize) {
        self.add(QuantumTransformation {
            gate: Gate::X,
            target,
            controls: Controls::Ones(vec![control1, control2]),
        });
    }

    /// Add the Hadamard (H) gate for a given target qubit to the list of QuantumTransformations
    #[inline]
    pub fn h(&mut self, target: usize) {
        self.add(QuantumTransformation {
            gate: Gate::H,
            target,
            controls: Controls::None,
        });
    }

    /// Add Ry gate for a given target qubit to the list of QuantumTransformations
    #[inline]
    pub fn ry(&mut self, angle: Float, target: usize) {
        self.add(QuantumTransformation {
            gate: Gate::RY(angle),
            target,
            controls: Controls::None,
        });
    }

    /// Add CRy gate for a given target qubit and a given control qubit to the list of
    /// QuantumTransformations
    #[inline]
    pub fn cry(&mut self, angle: Float, control: usize, target: usize) {
        self.add(QuantumTransformation {
            gate: Gate::RY(angle),
            target,
            controls: Controls::Single(control),
        });
    }

    /// Add Phase (P) gate for a given target qubit to the list of QuantumTransformations
    #[inline]
    pub fn p(&mut self, angle: Float, target: usize) {
        self.add(QuantumTransformation {
            gate: Gate::P(angle),
            target,
            controls: Controls::None,
        });
    }

    /// Add the Controlled Phase (CP) gate for a given target qubit and a given control qubit to
    /// the list of QuantumTransformations
    #[inline]
    pub fn cp(&mut self, angle: Float, control: usize, target: usize) {
        self.add(QuantumTransformation {
            gate: Gate::P(angle),
            target,
            controls: Controls::Single(control),
        });
    }

    /// Add Z gate for a given target qubit to the list of QuantumTransformations
    #[inline]
    pub fn z(&mut self, target: usize) {
        self.add(QuantumTransformation {
            gate: Gate::Z,
            target,
            controls: Controls::None,
        });
    }

    /// Add Rz gate for a given target qubit to the list of QuantumTransformations
    #[inline]
    pub fn rz(&mut self, angle: Float, target: usize) {
        self.add(QuantumTransformation {
            gate: Gate::RZ(angle),
            target,
            controls: Controls::None,
        });
    }

    /// Add all transformations for an inverse Quantum Fourier Transform to the list of QuantumTransformations
    #[inline]
    pub fn iqft(&mut self, targets: &[usize]) {
        for j in (0..targets.len()).rev() {
            self.h(targets[j]);
            for k in (0..j).rev() {
                self.cp(-PI / pow2f(j - k), targets[j], targets[k]);
            }
        }
    }

    /// Add a given `QuantumTransformation` to the list of transformations
    #[inline]
    pub fn add(&mut self, transformation: QuantumTransformation) {
        self.transformations.push(transformation)
    }

    /// Run the list of transformations against the State
    pub fn execute(&mut self) {
        for tr in self.transformations.drain(..) {
            match (&tr.controls, tr.gate) {
                (Controls::None, Gate::RZ(theta)) => {
                    apply(Gate::RZ(theta), &mut self.state, tr.target);
                }
                (Controls::None, Gate::RX(theta)) => {
                    apply(Gate::RX(theta), &mut self.state, tr.target);
                }
                (Controls::None, Gate::X) => {
                    apply(Gate::X, &mut self.state, tr.target);
                }
                (Controls::None, Gate::RY(theta)) => {
                    apply(Gate::RY(theta), &mut self.state, tr.target);
                }
                (Controls::None, Gate::Z) => {
                    apply(Gate::Z, &mut self.state, tr.target);
                }
                (Controls::None, Gate::H) => {
                    apply(Gate::H, &mut self.state, tr.target);
                }
                (Controls::None, Gate::P(theta)) => {
                    apply(Gate::P(theta), &mut self.state, tr.target);
                }
                (Controls::Single(control), Gate::X) => {
                    c_apply(Gate::X, &mut self.state, *control, tr.target);
                }
                (Controls::Single(control), Gate::P(theta)) => {
                    c_apply(Gate::P(theta), &mut self.state, *control, tr.target);
                }
                (Controls::Single(control), Gate::RY(theta)) => {
                    c_apply(Gate::RY(theta), &mut self.state, *control, tr.target);
                }
                (Controls::Ones(controls), Gate::X) => {
                    cc_apply(
                        Gate::X,
                        &mut self.state,
                        controls[0],
                        controls[1],
                        tr.target,
                    );
                }
                (Controls::None, Gate::M) => {
                    if !self.qubit_tracker.is_qubit_measured(tr.target) {
                        let value = measure_qubit(&mut self.state, tr.target, true, None);
                        self.qubit_tracker.set_measured_qubit(tr.target);
                        self.qubit_tracker
                            .set_val_for_measured_qubit(tr.target, value);
                    }
                }
                _ => {
                    todo!();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        math::modulus,
        utils::{assert_float_closeness, gen_random_state},
    };

    #[test]
    fn circuit() {
        let n = 2;
        let mut q = QuantumRegister::new(n);
        let mut a = QuantumRegister::new(1);
        let mut qc = QuantumCircuit::new2(&mut q, &mut a);

        for i in 0..n {
            qc.ry(PI / 2.0, q[i]);
        }
        qc.cry(PI / 2.0, q[0], q[1]);
        qc.p(PI / 2.0, q[0]);
        qc.cp(PI / 2.0, q[0], q[1]);
        qc.h(a[0]);

        qc.execute();
    }

    #[test]
    fn value_encoding() {
        let v = 2.4;
        let n = 3;

        let mut q = QuantumRegister::new(n);
        let mut qc = QuantumCircuit::new(&mut q);

        for i in 0..n {
            qc.h(q[i])
        }

        for i in 0..n {
            qc.p((2 as Float) * PI / pow2f(i + 1) * v, q[i])
        }

        let targets: Vec<usize> = (0..n).rev().collect();
        qc.iqft(&targets);
        qc.execute();
    }

    #[test]
    fn z_gate() {
        let n = 2;
        let mut q = QuantumRegister::new(n);
        let mut qc = QuantumCircuit::new(&mut q);

        for i in 0..n {
            qc.h(q[i])
        }

        qc.z(q[0]);

        qc.execute();
    }

    #[test]
    fn ccx() {
        let n = 3;
        let mut q = QuantumRegister::new(n);
        let mut qc = QuantumCircuit::new(&mut q);

        for i in 0..n - 1 {
            qc.h(q[i])
        }

        qc.ccx(q[0], q[1], q[2]);

        qc.execute();
        let _state = qc.get_statevector();
    }

    #[test]
    fn x_gate() {
        let n = 2;
        let mut q = QuantumRegister::new(n);
        let mut qc = QuantumCircuit::new(&mut q);

        for i in 0..n {
            qc.h(q[i])
        }

        qc.rx(PI / 2.0, q[0]);
        qc.execute();
    }

    #[test]
    fn measure() {
        const N: usize = 21;
        let state = gen_random_state(N);

        let sum = state
            .reals
            .iter()
            .zip(state.imags.iter())
            .map(|(re, im)| modulus(*re, *im).powi(2))
            .sum();

        // Make sure the generated random state is sound
        assert_float_closeness(sum, 1.0, 0.001);

        let mut qc = QuantumCircuit {
            state,
            transformations: Vec::new(),
            qubit_tracker: QubitTracker::new(),
        };

        // Add Measure gates for all the qubits
        for target in 0..N {
            qc.measure(target);
        }

        // Execute the circuit
        qc.execute();

        // Allocate stack storage of measured qubit values
        let mut measured_vals = [0; N];

        // Now collect the measured values
        for target in 0..N {
            let val = qc.qubit_tracker.get_qubit_measured_val(target);
            measured_vals[target] = val;
        }

        // Now we need to measure again...

        // Add Measure gates for all the qubits
        for target in 0..N {
            qc.measure(target);
        }

        // Execute the circuit
        qc.execute();

        // Now check that the new measured values are the same as what we got before
        for target in 0..N {
            assert!(
                qc.qubit_tracker.is_qubit_measured(target),
                "qubit {target} was already measured, but it wasn't marked as measured"
            );
            assert_eq!(
                measured_vals[target],
                qc.qubit_tracker.get_qubit_measured_val(target)
            );
        }
    }
}
