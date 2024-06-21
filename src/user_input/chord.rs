//! This module contains [`ButtonlikeChord`] and its impls.

use bevy::prelude::Reflect;
use leafwing_input_manager_macros::serde_typetag;
use serde::{Deserialize, Serialize};

use crate as leafwing_input_manager;
use crate::axislike::DualAxisData;
use crate::clashing_inputs::BasicInputs;
use crate::input_streams::InputStreams;
use crate::raw_inputs::RawInputs;
use crate::user_input::{Buttonlike, InputControlKind, UserInput};

use super::{Axislike, DualAxislike};

/// A combined input that groups multiple [`Buttonlike`]s together,
/// allowing you to define complex input combinations like hotkeys, shortcuts, and macros.
///
/// # Behaviors
///
/// - Activation: All included inputs must be active simultaneously.
/// - Deduplication: Adding duplicate inputs within a chord will ignore the extras,
///     preventing redundant data fetching.
///
/// ```rust
/// use bevy::prelude::*;
/// use bevy::input::InputPlugin;
/// use leafwing_input_manager::plugin::AccumulatorPlugin;
/// use leafwing_input_manager::prelude::*;
///
/// let mut app = App::new();
/// app.add_plugins((InputPlugin, AccumulatorPlugin));
///
/// // Define a chord using A and B keys
/// let input = ButtonlikeChord::new([KeyCode::KeyA, KeyCode::KeyB]);
///
/// // Pressing only one key doesn't activate the input
/// app.press_input(KeyCode::KeyA);
/// app.update();
/// assert!(!app.pressed(input.clone()));
///
/// // Pressing both keys activates the input
/// app.press_input(KeyCode::KeyB);
/// app.update();
/// assert!(app.pressed(input.clone()));
///
/// // Define a new chord with both axes for mouse movement.
/// let input = input.with_multiple([MouseMoveAxis::X, MouseMoveAxis::Y]);
///
/// // Note that this chord only reports a combined single-axis value.
/// // because it constructed from two single-axis inputs, not one dual-axis input.
/// app.send_axis_values(MouseMove::default(), [2.0, 3.0]);
/// app.update();
/// assert_eq!(app.read_axis_values(input.clone()), [5.0]);
///
/// // Define a new chord with two dual-axis inputs.
/// let input = input.with(MouseMove::default()).with(MouseScroll::default());
///
/// // Note that this chord only reports the value from the first included dual-axis input.
/// app.send_axis_values(MouseMove::default(), [2.0, 3.0]);
/// app.send_axis_values(MouseScroll::default(), [4.0, 5.0]);
/// app.update();
/// assert_eq!(app.read_axis_values(input), [2.0, 3.0]);
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[must_use]
pub struct ButtonlikeChord(
    // Note: We can't use a HashSet here because of
    // https://users.rust-lang.org/t/hash-not-implemented-why-cant-it-be-derived/92416/8
    // We can't use a BTreeSet because the underlying types don't impl Ord
    // We don't want to use a PetitSet here because of memory bloat
    // So a vec it is!
    pub(crate) Vec<Box<dyn Buttonlike>>,
);

impl ButtonlikeChord {
    /// Creates a [`ButtonlikeChord`] from multiple [`Buttonlike`]s, avoiding duplicates.
    /// Note that all elements within the iterator must be of the same type (homogeneous).
    /// You can still use other methods to add different types of inputs into the chord.
    ///
    /// This ensures that the same input isn't added multiple times,
    /// preventing redundant data fetching from multiple instances of the same input.
    #[inline]
    pub fn new<U: Buttonlike>(inputs: impl IntoIterator<Item = U>) -> Self {
        Self::default().with_multiple(inputs)
    }

    /// Creates a [`ButtonlikeChord`] that only contains the given [`Buttonlike`].
    /// You can still use other methods to add different types of inputs into the chord.
    #[inline]
    pub fn from_single(input: impl Buttonlike) -> Self {
        Self::default().with(input)
    }

    /// Adds the given [`Buttonlike`] into this chord, avoiding duplicates.
    ///
    /// This ensures that the same input isn't added multiple times,
    /// preventing redundant data fetching from multiple instances of the same input.
    #[inline]
    pub fn with(mut self, input: impl Buttonlike) -> Self {
        self.push_boxed_unique(Box::new(input));
        self
    }

    /// Adds multiple [`Buttonlike`]s into this chord, avoiding duplicates.
    /// Note that all elements within the iterator must be of the same type (homogeneous).
    ///
    /// This ensures that the same input isn't added multiple times,
    /// preventing redundant data fetching from multiple instances of the same input.
    #[inline]
    pub fn with_multiple<U: Buttonlike>(mut self, inputs: impl IntoIterator<Item = U>) -> Self {
        for input in inputs.into_iter() {
            self.push_boxed_unique(Box::new(input));
        }
        self
    }

    /// Adds the given boxed dyn [`Buttonlike`] to this chord, avoiding duplicates.
    ///
    /// This ensures that the same input isn't added multiple times,
    /// preventing redundant data fetching from multiple instances of the same input.
    #[inline]
    fn push_boxed_unique(&mut self, input: Box<dyn Buttonlike>) {
        if !self.0.contains(&input) {
            self.0.push(input);
        }
    }
}

#[serde_typetag]
impl UserInput for ButtonlikeChord {
    /// [`ButtonlikeChord`] acts as a virtual button.
    #[inline]
    fn kind(&self) -> InputControlKind {
        InputControlKind::Button
    }

    /// Retrieves a list of simple, atomic [`Buttonlike`]s that compose the chord.
    #[inline]
    fn decompose(&self) -> BasicInputs {
        let inputs = self
            .0
            .iter()
            .flat_map(|input| input.decompose().inputs())
            .collect();
        BasicInputs::Group(inputs)
    }

    /// Returns the [`RawInputs`] that combines the raw input events of all inner inputs.
    #[inline]
    fn raw_inputs(&self) -> RawInputs {
        self.0.iter().fold(RawInputs::default(), |inputs, next| {
            inputs.merge_input(&next.raw_inputs())
        })
    }
}

impl Buttonlike for ButtonlikeChord {
    /// Checks if all the inner inputs within the chord are active simultaneously.
    #[must_use]
    #[inline]
    fn pressed(&self, input_streams: &InputStreams) -> bool {
        self.0.iter().all(|input| input.pressed(input_streams))
    }
}

impl<U: Buttonlike> FromIterator<U> for ButtonlikeChord {
    /// Creates a [`ButtonlikeChord`] from an iterator over multiple [`Buttonlike`]s, avoiding duplicates.
    /// Note that all elements within the iterator must be of the same type (homogeneous).
    /// You can still use other methods to add different types of inputs into the chord.
    ///
    /// This ensures that the same input isn't added multiple times,
    /// preventing redundant data fetching from multiple instances of the same input.
    #[inline]
    fn from_iter<T: IntoIterator<Item = U>>(iter: T) -> Self {
        Self::default().with_multiple(iter)
    }
}

/// A combined input that groups a [`Buttonlike`] and a [`Axislike`] together,
/// allowing you to only read the axis value when the button is pressed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[must_use]
pub struct AxislikeChord {
    /// The button that must be pressed to read the axis value.
    pub button: Box<dyn Buttonlike>,
    /// The axis value that is read when the button is pressed.
    pub axis: Box<dyn Axislike>,
}

impl AxislikeChord {
    /// Creates a new [`AxislikeChord`] from the given [`Buttonlike`] and [`Axislike`].
    #[inline]
    pub fn new(button: impl Buttonlike, axis: impl Axislike) -> Self {
        Self {
            button: Box::new(button),
            axis: Box::new(axis),
        }
    }
}

#[serde_typetag]
impl UserInput for AxislikeChord {
    /// [`AxislikeChord`] acts as a virtual axis.
    #[inline]
    fn kind(&self) -> InputControlKind {
        InputControlKind::Axis
    }

    /// Retrieves a list of simple, atomic [`Buttonlike`]s that compose the chord.
    #[inline]
    fn decompose(&self) -> BasicInputs {
        let button_input = self.button.to_user_input();
        let axis_input = self.axis.to_user_input();
        let inputs = vec![button_input, axis_input];

        BasicInputs::Group(inputs)
    }

    /// Returns the [`RawInputs`] that combines the raw input events of all inner inputs.
    #[inline]
    fn raw_inputs(&self) -> RawInputs {
        let raw_button_input = self.button.raw_inputs();
        let raw_axis_input = self.axis.raw_inputs();

        raw_button_input.merge_input(&raw_axis_input)
    }
}

impl Axislike for AxislikeChord {
    fn value(&self, input_streams: &InputStreams) -> f32 {
        if self.button.pressed(input_streams) {
            self.axis.value(input_streams)
        } else {
            0.0
        }
    }
}

/// A combined input that groups a [`Buttonlike`] and a [`DualAxislike`] together,
/// allowing you to only read the dual axis data when the button is pressed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[must_use]
pub struct DualAxislikeChord {
    /// The button that must be pressed to read the axis value.
    pub button: Box<dyn Buttonlike>,
    /// The dual axis data that is read when the button is pressed.
    pub dual_axis: Box<dyn DualAxislike>,
}

impl DualAxislikeChord {
    /// Creates a new [`AxislikeChord`] from the given [`Buttonlike`] and [`Axislike`].
    #[inline]
    pub fn new(button: impl Buttonlike, dual_axis: impl DualAxislike) -> Self {
        Self {
            button: Box::new(button),
            dual_axis: Box::new(dual_axis),
        }
    }
}

#[serde_typetag]
impl UserInput for DualAxislikeChord {
    /// [`DualAxislikeChord`] acts as a virtual dual-axis.
    #[inline]
    fn kind(&self) -> InputControlKind {
        InputControlKind::DualAxis
    }

    /// Retrieves a list of simple, atomic [`Buttonlike`]s that compose the chord.
    #[inline]
    fn decompose(&self) -> BasicInputs {
        let button_input = self.button.to_user_input();
        let dual_axis_input = self.dual_axis.to_user_input();
        let inputs = vec![button_input, dual_axis_input];

        BasicInputs::Group(inputs)
    }

    /// Returns the [`RawInputs`] that combines the raw input events of all inner inputs.
    #[inline]
    fn raw_inputs(&self) -> RawInputs {
        let raw_button_input = self.button.raw_inputs();
        let raw_axis_input = self.dual_axis.raw_inputs();

        raw_button_input.merge_input(&raw_axis_input)
    }
}

impl DualAxislike for DualAxislikeChord {
    fn axis_pair(&self, input_streams: &InputStreams) -> DualAxisData {
        if self.button.pressed(input_streams) {
            self.dual_axis.axis_pair(input_streams)
        } else {
            DualAxisData::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::input::gamepad::{
        GamepadConnection, GamepadConnectionEvent, GamepadEvent, GamepadInfo,
    };
    use bevy::input::InputPlugin;
    use bevy::prelude::*;

    use super::*;
    use crate::axislike::DualAxisData;
    use crate::plugin::AccumulatorPlugin;
    use crate::prelude::*;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(InputPlugin)
            .add_plugins(AccumulatorPlugin);

        // WARNING: you MUST register your gamepad during tests,
        // or all gamepad input mocking actions will fail
        let mut gamepad_events = app.world_mut().resource_mut::<Events<GamepadEvent>>();
        gamepad_events.send(GamepadEvent::Connection(GamepadConnectionEvent {
            // This MUST be consistent with any other mocked events
            gamepad: Gamepad { id: 1 },
            connection: GamepadConnection::Connected(GamepadInfo {
                name: "TestController".into(),
            }),
        }));

        // Ensure that the gamepad is picked up by the appropriate system
        app.update();
        // Ensure that the connection event is flushed through
        app.update();
        app
    }

    fn check(
        input: &impl Buttonlike,
        input_streams: &InputStreams,
        expected_pressed: bool,
        expected_value: f32,
        expected_axis_pair: Option<DualAxisData>,
    ) {
        assert_eq!(input.pressed(input_streams), expected_pressed);
        assert_eq!(input.value(input_streams), expected_value);
        assert_eq!(input.axis_pair(input_streams), expected_axis_pair);
    }

    fn pressed(input: &impl Buttonlike, input_streams: &InputStreams) {
        check(input, input_streams, true, 1.0, None);
    }

    fn released(input: &impl Buttonlike, input_streams: &InputStreams) {
        check(input, input_streams, false, 0.0, None);
    }

    #[test]
    fn test_chord_with_buttons_only() {
        let chord = ButtonlikeChord::new([KeyCode::KeyC, KeyCode::KeyH])
            .with(KeyCode::KeyO)
            .with_multiple([KeyCode::KeyR, KeyCode::KeyD]);

        let required_keys = [
            KeyCode::KeyC,
            KeyCode::KeyH,
            KeyCode::KeyO,
            KeyCode::KeyR,
            KeyCode::KeyD,
        ];

        let expected_inners = required_keys
            .iter()
            .map(|key| Box::new(*key) as Box<dyn Buttonlike>)
            .collect::<Vec<_>>();
        assert_eq!(chord.0, expected_inners);

        let expected_raw_inputs = RawInputs::from_keycodes(required_keys);
        assert_eq!(chord.raw_inputs(), expected_raw_inputs);

        // No keys pressed, resulting in a released chord with a value of zero.
        let mut app = test_app();
        app.update();
        let inputs = InputStreams::from_world(app.world(), None);
        released(&chord, &inputs);

        // All required keys pressed, resulting in a pressed chord with a value of one.
        let mut app = test_app();
        for key in required_keys {
            app.press_input(key);
        }
        app.update();
        let inputs = InputStreams::from_world(app.world(), None);
        pressed(&chord, &inputs);

        // Some required keys pressed, but not all required keys for the chord,
        // resulting in a released chord with a value of zero.
        for i in 1..=4 {
            let mut app = test_app();
            for key in required_keys.iter().take(i) {
                app.press_input(*key);
            }
            app.update();
            let inputs = InputStreams::from_world(app.world(), None);
            released(&chord, &inputs);
        }

        // Five keys pressed, but not all required keys for the chord,
        // resulting in a released chord with a value of zero.
        let mut app = test_app();
        for key in required_keys.iter().take(4) {
            app.press_input(*key);
        }
        app.press_input(KeyCode::KeyB);
        app.update();
        let inputs = InputStreams::from_world(app.world(), None);
        released(&chord, &inputs);
    }

    #[test]
    fn test_chord_with_buttons_and_axes() {
        use crate::axislike::DualAxisData;

        let chord = ButtonlikeChord::new([KeyCode::KeyA, KeyCode::KeyB])
            .with(MouseScrollAxis::X)
            .with(MouseScrollAxis::Y)
            .with(GamepadStick::LEFT)
            .with(GamepadStick::RIGHT);

        let required_keys = [KeyCode::KeyA, KeyCode::KeyB];

        let expected_inners = required_keys
            .iter()
            .map(|key| Box::new(*key) as Box<dyn Buttonlike>)
            .chain(Some(Box::new(MouseScrollAxis::X) as Box<dyn Buttonlike>))
            .chain(Some(Box::new(MouseScrollAxis::Y) as Box<dyn Buttonlike>))
            .chain(Some(Box::new(GamepadStick::LEFT) as Box<dyn Buttonlike>))
            .chain(Some(Box::new(GamepadStick::RIGHT) as Box<dyn Buttonlike>))
            .collect::<Vec<_>>();
        assert_eq!(chord.0, expected_inners);

        let expected_raw_inputs = RawInputs::from_keycodes(required_keys)
            .merge_input(&MouseScrollAxis::X.raw_inputs())
            .merge_input(&MouseScrollAxis::Y.raw_inputs())
            .merge_input(&GamepadStick::LEFT.raw_inputs())
            .merge_input(&GamepadStick::RIGHT.raw_inputs());
        assert_eq!(chord.raw_inputs(), expected_raw_inputs);

        // No input events, resulting in a released chord with values of zeros.
        let zeros = Some(DualAxisData::default());
        let mut app = test_app();
        app.update();
        let inputs = InputStreams::from_world(app.world(), None);
        check(&chord, &inputs, false, 0.0, zeros);

        // Only one or all required keys pressed without axial inputs,
        // resulting in a released chord with values of zeros.
        for i in 1..=2 {
            let mut app = test_app();
            for key in required_keys.iter().take(i) {
                app.press_input(*key);
            }
            app.update();
            let inputs = InputStreams::from_world(app.world(), None);
            check(&chord, &inputs, false, 0.0, zeros);
        }

        // Send changes in values of some required single-axis inputs,
        // resulting in a released chord with a combined value from the given single-axis values.
        let value = 2.0;
        let mut app = test_app();
        app.send_axis_values(MouseScrollAxis::X, [value]);
        app.update();
        let inputs = InputStreams::from_world(app.world(), None);
        check(&chord, &inputs, false, value, zeros);

        let data = DualAxisData::new(2.0, 3.0);
        let mut app = test_app();
        app.send_axis_values(MouseScroll::default(), [data.x(), data.y()]);
        app.update();
        let inputs = InputStreams::from_world(app.world(), None);
        check(&chord, &inputs, false, data.x() + data.y(), zeros);

        // Send changes in values of first dual-axis input,
        // resulting in a released chord with the given value.
        let data = DualAxisData::new(0.5, 0.6);
        let mut app = test_app();
        app.send_axis_values(GamepadStick::LEFT, [data.x(), data.y()]);
        app.update();
        let inputs = InputStreams::from_world(app.world(), None);
        check(&chord, &inputs, false, 0.0, Some(data));

        // Send changes in values of second dual-axis input,
        // resulting in a released chord with values of zeros.
        let data = DualAxisData::new(0.5, 0.6);
        let mut app = test_app();
        app.send_axis_values(GamepadStick::RIGHT, [data.x(), data.y()]);
        app.update();
        let inputs = InputStreams::from_world(app.world(), None);
        check(&chord, &inputs, false, 0.0, zeros);

        // Send changes in values of first dual-axis input and all single-axis inputs,
        // resulting in a released chord with a combined value from the given single-axis values
        // and an axis pair of the first dual-axis input.
        let data = DualAxisData::new(0.5, 0.6);
        let single = Vec2::new(0.8, -0.2);
        let mut app = test_app();
        app.send_axis_values(GamepadStick::LEFT, [data.x(), data.y()]);
        app.send_axis_values(MouseScrollAxis::X, [single.x]);
        app.send_axis_values(MouseScrollAxis::Y, [single.y]);
        app.update();
        let inputs = InputStreams::from_world(app.world(), None);
        check(&chord, &inputs, false, single.x + single.y, Some(data));

        // The chord are pressed only if all inputs are activated.
        let first_data = DualAxisData::new(0.5, 0.6);
        let second_data = DualAxisData::new(0.4, 0.8);
        let single = Vec2::new(0.8, -0.2);
        let mut app = test_app();
        for key in required_keys {
            app.press_input(key);
        }
        app.send_axis_values(GamepadStick::LEFT, [first_data.x(), first_data.y()]);
        app.send_axis_values(GamepadStick::RIGHT, [second_data.x(), second_data.y()]);
        app.send_axis_values(MouseScrollAxis::X, [single.x]);
        app.send_axis_values(MouseScrollAxis::Y, [single.y]);
        app.update();
        let inputs = InputStreams::from_world(app.world(), None);
        check(&chord, &inputs, true, single.x + single.y, Some(first_data));
    }
}
