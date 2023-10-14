use std::{
    collections::{HashMap, HashSet},
    str::Utf8Error,
    sync::Arc,
};

use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};

use super::{Field, Msg, MsgId, Repitition, Type};

static PRIMITIVE_TYPES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "bool", "byte", "char", "float32", "float64", "int8", "uint8", "int16", "uint16", "int32",
        "uint32", "int64", "uint64", "string", "wstring",
    ])
});
static RE_DEF_SPLIT: Lazy<Regex> = Lazy::new(|| {
    RegexBuilder::new(r"^={3,}$")
        .multi_line(true)
        .build()
        .unwrap()
});
static RE_MSG: Lazy<Regex> = Lazy::new(|| {
    RegexBuilder::new(r"^MSG:\s+(\S+)$")
        .multi_line(true)
        .build()
        .unwrap()
});
static FIELD_TYPE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([A-Za-z0-9_/]+)(\[(<=)?(\d+)?\])?$").unwrap());

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("message name `{0}` is invalid")]
    InvalidMessageName(String),
    #[error("Cannot find message name from `{0}`")]
    MissingMessageName(String),
    #[error("Cannot find message definition for `{0:?}`")]
    MissingMessageDefinition(MsgId),
    #[error("Field definition `{0}` is invalid")]
    InvalidFieldDefinition(String),
    #[error("Field type `{0}` is invalid")]
    InvalidFieldType(String),
    #[error("Schema test is not valid UTF8: {0}")]
    SchemaTextUtf8(Utf8Error),
}

pub fn get<S>(
    schema_name: &str,
    message_table: &HashMap<MsgId, Arc<Msg>, S>,
) -> Result<Option<Arc<Msg>>, Error>
where
    S: std::hash::BuildHasher,
{
    let schema_id = parse_name_parts(schema_name)?;
    Ok(message_table.get(&schema_id).cloned())
}

pub fn parse<S>(
    schema_name: &str,
    schema_text: &[u8],
    message_table: &mut HashMap<MsgId, Arc<Msg>, S>,
) -> Result<Arc<Msg>, Error>
where
    S: std::hash::BuildHasher,
{
    let schema_id = parse_name_parts(schema_name)?;
    if let Some(message) = message_table.get(&schema_id) {
        return Ok(message.clone());
    }

    let schema_text = std::str::from_utf8(schema_text).map_err(Error::SchemaTextUtf8)?;

    let mut unparsed_table = HashMap::new();
    let mut message_texts = RE_DEF_SPLIT.split(schema_text);

    // first message is special because it does not have a `MSG: <name>` line.
    if let Some(message_text) = message_texts.next() {
        let message_text = message_text.trim().to_owned();
        unparsed_table.insert(schema_id.clone(), message_text);
    } else {
        unreachable!("split has at least one element");
    }

    for message_text in message_texts {
        let message_text = message_text.trim();
        let captures = RE_MSG
            .captures(message_text)
            .ok_or_else(|| Error::MissingMessageName(message_text.to_owned()))?;
        let message_id = parse_name_parts(&captures[1])?;
        if message_table.contains_key(&message_id) {
            continue;
        }

        let message_text = RE_MSG.replace(message_text, "").to_string();
        unparsed_table.insert(message_id, message_text);
    }

    parse_message(&schema_id, message_table, &mut unparsed_table)
}

fn parse_message<S1, S2>(
    message_id: &MsgId,
    message_table: &mut HashMap<MsgId, Arc<Msg>, S1>,
    unparsed_table: &mut HashMap<MsgId, String, S2>,
) -> Result<Arc<Msg>, Error>
where
    S1: std::hash::BuildHasher,
    S2: std::hash::BuildHasher,
{
    if let Some(message) = message_table.get(message_id) {
        return Ok(message.clone());
    }

    let message_text = unparsed_table
        .remove(message_id)
        .ok_or_else(|| Error::MissingMessageDefinition(message_id.clone()))?;
    let mut message = Msg { fields: Vec::new() };
    for line in message_text.lines().filter(|s| {
        let s = s.trim();
        !s.is_empty() && !s.starts_with('#')
    }) {
        let line = line
            .split('#')
            .next()
            .expect("split always returns at least one element");
        let field = parse_field(&message_id.package, line, |id| {
            parse_message(&id, message_table, unparsed_table)
        })?;
        if let Some(field) = field {
            message.fields.push(field);
        }
    }

    let message = Arc::new(message);
    message_table.insert(message_id.clone(), message.clone());
    Ok(message)
}

fn parse_field(
    package: &str,
    line: &str,
    mut custom_msg_resolver: impl FnMut(MsgId) -> Result<Arc<Msg>, Error>,
) -> Result<Option<Field>, Error> {
    let (field_type, remaining) = line
        .split_once(' ')
        .ok_or(Error::InvalidFieldDefinition(line.to_owned()))?;

    if let Some((field_name, field_value)) = remaining.split_once('=') {
        log::debug!("skip constant: {field_name}: {field_type} = {field_value}");
        return Ok(None);
    }

    let field_type = field_type.trim();
    let field_name = remaining
        .split_whitespace()
        .find(|s| !s.is_empty())
        .unwrap()
        .trim();
    let captures = FIELD_TYPE_PATTERN
        .captures(field_type)
        .ok_or(Error::InvalidFieldType(field_type.to_owned()))?;
    let repitition = match (captures.get(2), captures.get(3), captures.get(4)) {
        (None, _, _) => Repitition::Single,
        (Some(_), None, None) => Repitition::Unbounded,
        (Some(_), None, Some(count)) => Repitition::Fixed(count.as_str().parse().unwrap()),
        (Some(_), Some(_), Some(count)) => Repitition::Bounded(count.as_str().parse().unwrap()),
        _ => return Err(Error::InvalidFieldType(field_type.to_owned())),
    };
    let field_unit_type = &captures[1];
    let field = if PRIMITIVE_TYPES.contains(&field_unit_type) {
        let ty = match field_unit_type {
            "bool" => Type::Bool,
            "byte" | "int8" => Type::I8,
            "char" | "uint8" => Type::U8,
            "float32" => Type::F32,
            "float64" => Type::F64,
            "int16" => Type::I16,
            "uint16" => Type::U16,
            "int32" => Type::I32,
            "uint32" => Type::U32,
            "int64" => Type::I64,
            "uint64" => Type::U64,
            "string" => Type::Str,
            "wstring" => unimplemented!("wstring is not supported"),
            _ => unreachable!(),
        };
        Field {
            name: field_name.to_owned(),
            ty,
            repitition,
        }
    } else {
        let field_id = parse_name_parts(field_unit_type)
            .unwrap_or_else(|_| MsgId::new(package, field_unit_type));
        let msg = custom_msg_resolver(field_id)?;
        Field {
            name: field_name.to_owned(),
            ty: Type::Msg(msg),
            repitition,
        }
    };
    Ok(Some(field))
}

fn parse_name_parts(fullname: &str) -> Result<MsgId, Error> {
    let mut name_parts = fullname.splitn(4, '/');
    match (
        name_parts.next(),
        name_parts.next(),
        name_parts.next(),
        name_parts.next(),
    ) {
        (Some(package), Some("msg"), Some(name), None)
        | (Some(package), Some(name), None, None) => Ok(MsgId::new(package, name)),
        _ => Err(Error::InvalidMessageName(fullname.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_nested_schema() {
        let schema_name = "firmware_common/msg/DCMotorGroupCommand";
        let schema_text = br#"
# TODO: dc motor group is almost useless, because most of the dc motors act on its own. Remove this later.
# Commands for DC motors (controlled by a PWM signal and a direction pin)
DCMotorCommand[1] motors
# Number of dc motors
uint8 motors_count

================================================================================
MSG: firmware_common/DCMotorCommand
# The percent of command velocity over max absolute velocity. The range is [-100, 100].
int8 vel_percent
# The time to keep this velocity before resetting to zero. The unit is millisecond.
uint16 timeout_ms
        "#;
        let mut type_table = HashMap::new();
        let result = parse(schema_name, schema_text, &mut type_table);
        let schema = result.unwrap();
        assert_eq!(schema.fields.len(), 2);
        assert_eq!(schema.fields[0].name, "motors");
        assert_eq!(
            schema.fields[0].ty,
            Type::Msg(Arc::new(Msg {
                fields: vec![
                    Field {
                        name: "vel_percent".to_owned(),
                        ty: Type::I8,
                        repitition: Repitition::Single,
                    },
                    Field {
                        name: "timeout_ms".to_owned(),
                        ty: Type::U16,
                        repitition: Repitition::Single,
                    },
                ],
            }))
        );
        assert_eq!(schema.fields[0].repitition, Repitition::Fixed(1));
        assert_eq!(schema.fields[1].name, "motors_count");
        assert_eq!(schema.fields[1].ty, Type::U8);
        assert_eq!(schema.fields[1].repitition, Repitition::Single);
        assert_eq!(type_table.len(), 2);
        assert!(type_table.contains_key(&MsgId::new("firmware_common", "DCMotorGroupCommand")));
        assert!(type_table.contains_key(&MsgId::new("firmware_common", "DCMotorCommand")));
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn test_very_long_schema() {
        _ = env_logger::try_init();
        let schema_name = "mobility_interface/msg/MobilityV4Telemetry";
        let schema_text = br#"
# Feedback from the traction submodule
TractionTelemetry traction_submodule
# Feedback from the pivot submodule
PivotTelemetry pivot_submodule
# Feedback from the lift submodule
LiftV4Telemetry lift_submodule
# Feedback from positive x binlift submodule
BinliftTelemetry positive_x_binlift_submodule
# Feedback from negative x binlift submodule
BinliftTelemetry negative_x_binlift_submodule
# Feedback from positive x latch submodule
LatchTelemetry positive_x_latch_submodule
# Feedback from negative x latch submodule
LatchTelemetry negative_x_latch_submodule

# The whole firmware's version
firmware_common/SystemVersion system_version
# Feedback related to reset
firmware_common/ResetFeedback reset_feedback
# Current timestamp on the firmware
builtin_interfaces/Time timestamp
# Battery status
firmware_common/BatteryFeedback battery
# Time passed since the heartbeat monitor received the last heartbeat
firmware_common/HeartbeatFeedback heartbeat
# LED feedback
firmware_common/LedUnitFeedback led_unit
================================================================================
MSG: builtin_interfaces/Time
# This message communicates ROS Time defined here:
# https://design.ros2.org/articles/clock_and_time.html

# The seconds component, valid over all int32 values.
int32 sec

# The nanoseconds component, valid in the range [0, 10e9).
uint32 nanosec

================================================================================
MSG: firmware_common/BatteryFeedback
# Message for feedback of battery status (V4)

# Bitmask of active GPIO pins
uint32  gpio_bitmask
# Battery current measurement in A
float32 pack_current_filtered_a
# Battery voltage measurement in V
float32 pack_voltage_v
# Battery state of charge in %
float32 pack_soc
# Battery remaining amp hours in Ah
float32 pack_remaining_cap_ah
# Shuntboard current reading for channel 1 in A
float32 shunt_current_a_1
# Shuntboard current reading for channel 2 in A
float32 shunt_current_a_2
# I2C OK true if healthy false if unhealthy
bool  i2c_ok
# CAN OK true if healthy false if unhealthy
bool  can_ok
# Hotswap version major
uint8  hotswap_version_major
# Hotswap version minor
uint8  hotswap_version_minor
# Hotswap version patch
uint8  hotswap_version_patch

================================================================================
MSG: firmware_common/HeartbeatFeedback
# The time passed on the firmware since it received the last heartbeat
uint16 ms_since_last

================================================================================
MSG: firmware_common/LedUnitFeedback
uint8[3] observations

================================================================================
MSG: firmware_common/ResetFeedback
uint32 cause

# https://docs.zephyrproject.org/apidoc/latest/hwinfo_8h.html
uint32 RESET_PIN_BIT = 0
uint32 RESET_SOFTWARE_BIT = 1
uint32 RESET_BROWNOUT_BIT = 2
uint32 RESET_POR_BIT = 3
uint32 RESET_WATCHDOG_BIT = 4
uint32 RESET_DEBUG_BIT = 5
uint32 RESET_SECURITY_BIT = 6
uint32 RESET_LOW_POWER_WAKE_BIT = 7
uint32 RESET_CPU_LOCKUP_BIT = 8
uint32 RESET_PARITY_BIT = 9
uint32 RESET_PLL_BIT = 10
uint32 RESET_CLOCK_BIT = 11
uint32 RESET_HARDWARE_BIT = 12
uint32 RESET_USER_BIT = 13
uint32 RESET_TEMPERATURE_BIT = 14
================================================================================
MSG: firmware_common/SystemVersion
uint8 major
uint8 minor
uint8 patch

================================================================================
MSG: mobility_interface/BinliftTelemetry
firmware_common/MotorGroup1Feedback motor_group
firmware_common/BinliftFSMFeedback binlift_fsm
firmware_common/CommandRecorder1Feedback command_recorder
firmware_common/ConverterResult1Feedback joint_group
firmware_common/TrajectoryGeneratorFeedback traj_generator
firmware_common/StallDetector1Feedback stall_detector
firmware_common/Calibration1Feedback calibration_manager
firmware_common/BrakeGroupFeedback brake_group
firmware_common/GrappleSensorFeedback grapple_sensor
# Binlift weight estimator
firmware_common/BinliftWeightEstimatorFeedback weight_estimator

================================================================================
MSG: firmware_common/BinliftFSMFeedback
uint32 enabled_mask
uint32 working_mask
uint8 state

uint8 INIT=0
uint8 DISABLING=1
uint8 DISABLED=2
uint8 ZEROING=3
uint8 ENABLING=4
uint8 TO_POS_MODE=5
uint8 IDLE=6
uint8 POS_PRE_MOVE=7
uint8 POS_MOVING=8
uint8 POS_STALLING=9
uint8 TO_CALIB_MODE=10
uint8 PRE_CALIBRATE=11
uint8 CALIBRATING=12
uint8 TO_VEL_MODE=13
uint8 VEL_PRE_MOVE=14
uint8 VEL_MOVING=15
uint8 POS_IGNORE_TENSION_PRE_MOVE=16
uint8 POS_IGNORE_TENSION_MOVING=17
uint8 TO_VEL_RECOVER_TENSION_MODE=18
uint8 VEL_RECOVER_TENSION_PRE_MOVE=19
uint8 VEL_RECOVER_TENSION_MOVING=20

uint8 VERSION_ERROR=254
uint8 FATAL_ERROR=255

================================================================================
MSG: firmware_common/BinliftWeightEstimatorFeedback
# The estimation of the weight based on the data since the start of the trajectory
float32 weight_kg
# The estimation of the weight based on the current cycle's data
float32 weight_kg_this_cycle
# Number of points used for binlift weight estimation for the current trajectory
uint32 num_estimation_points

##### Debug info #####
float32 filtered_position
float32 filtered_velocity
float32 filtered_torque
float32 feedback_acceleration
float32 command_acceleration
float32 filtered_command_acceleration
================================================================================
MSG: firmware_common/BrakeGroupFeedback
# TODO: the waste of space can be avoid in the same way as the motor group
BrakeFeedback[2] brakes
uint8 brakes_count

================================================================================
MSG: firmware_common/BrakeFeedback
uint8 gpio_level
uint8 estimator_released

================================================================================
MSG: firmware_common/Calibration1Feedback
JointCalibrationFeedback[1] joints

================================================================================
MSG: firmware_common/JointCalibrationFeedback
# A message representing the calibration result
#
# ## introduction
#
# The conversion between motor position and joint position is as follows.
#
# ```
#     coeff * (motor_position - motor_calib) = (joint_position - joint_calib)
#  => joint_position = coeff * motor_position + joint_calib - coeff * motor_calib
# ```
#
# where the `motor_calib` is the calibrated motor position and `joint_calib` is the joint position corresponding to
# this motor position.
#
# If we let `bias = joint_calib - coeff * motor_calib`, then the conversion will be as simple as an affine
# transform.
#
# ```
# joint_position = coeff * motor_position + bias
# motor_position = (joint_position - bias) / coeff
# ```
#
# ## How to get the `coeff` and `bias`
#
# Usually the `coeff` and `joint_calib` is determined by the mechanical design of the system, so they are passed as
# parameters to the calibration manager. What we need to find is the `motor_calib`.
#
# - For joints that skip calibration, `motor_calib` defaults to 0.0
# - For joints that need single-side calibration, `motor_calib` is the motor value when hitting the boundary
# - For joints that need double-side calibration, `motor_calib` is the center of the two boundaries found
#
# An extra safety checking is implemented for double-side calibration, which checks if the range between two boundaries
# is close to the range we desire. If the error between the range found and the range desired is too large, the
# `motor_calib` is left as unknown.

# The motor position for the upper limit in the joint space. 0.0 if not calibrated
float32 upper_limit
# Whether the upper limit is found
uint8   is_upper_limit_calibrated
# The motor position for the lower limit in the joint space. 0.0 if not calibrated
float32 lower_limit
# Whether the lower limit is found
uint8   is_lower_limit_calibrated
# The `bias` described above. 0.0 if not calibrated
float32 calibrated_limit
# Whether the whole calibration is done
uint8   is_calibrated

================================================================================
MSG: firmware_common/CommandRecorder1Feedback
LastCommand[1] last_commands

================================================================================
MSG: firmware_common/LastCommand
MotorState last_setpoint_command
MotorSpecialCommand last_special_command
PIDGain last_gain_command
uint8 is_vel_gain

# which command is sent in the last cycle
uint8 last_command_type
uint8 CMD_TYPE_NONE=0
uint8 CMD_TYPE_SPECIAL=1
uint8 CMD_TYPE_GAIN=2
uint8 CMD_TYPE_SETPOINT=3

# a bit flag for whether the last_XXX_command is valid.
# bit 0: whether the last_gain_command is valid
# bit 1: whether the last_setpoint_command is valid
# bit 2: whether the last_special_command is valid
uint8 has_commands
uint8 HAS_COMMAND_BIT_GAIN=0
uint8 HAS_COMMAND_BIT_SETPOINT=1
uint8 HAS_COMMAND_BIT_SPECIAL=2

================================================================================
MSG: firmware_common/MotorSpecialCommand
uint8 command

uint8 REQUEST_VERSION=0xFA
uint8 REQUEST_FEEDBACK=0xFF
uint8 ENABLE_MOTOR=0xFC
uint8 DISABLE_MOTOR=0xFD
uint8 ZERO_MOTOR=0xFE
uint8 ZERO_ROUND=0xFB

================================================================================
MSG: firmware_common/MotorState
float32 position
float32 velocity
float32 torque

================================================================================
MSG: firmware_common/PIDGain
float32 kp
float32 ki
float32 kd
================================================================================
MSG: firmware_common/ConverterResult1Feedback
MotorState[1] states

================================================================================
MSG: firmware_common/GrappleSensorFeedback
# Indicate whether binlift is tensioned
uint8 tension_state

uint8 UNTENSIONED = 0
uint8 TENSIONED = 1

# Voltage reading rom grapple sensor analog pin
uint32 voltage_reading_raw
uint32 voltage_reading_filtered
================================================================================
MSG: firmware_common/MotorGroup1Feedback
MotorFeedback[1] motor_feedbacks
uint8 master_error

================================================================================
MSG: firmware_common/MotorFeedback
# whether a state feedback is received in this cycle
uint8 has_state
# the last state feedback received. Note: if no new feedback is received in this cycle, old feedback is returned here
MotorState state

# whether a verbose feedback is received in this cycle
uint8 has_verbose
# the last verbose feedback received. Note: if no new feedback is received in this cycle, old feedback is returned here
MotorVerbose verbose

# whether a version feedback is received in this cycle
uint8 has_version
# the last version feedback received. Note: if no new feedback is received in this cycle, old feedback is returned here
MotorVersion version

# the number of CAN feedbacks that are lost
uint8 can_error_count
# the number of CAN feedbacks where the desired torque and the real torque do not match
uint8 torque_error_count
# whether the motor is enabled. Note: if no new feedback is received in this cycle, old feedback is returned here
uint8 is_enabled
# the number of failed transmissions in this cycle
uint8 failed_transmission_count
# can_id of the motor
uint8 can_id
================================================================================
MSG: firmware_common/MotorVerbose
float32 desired_torque
uint32 fault_status_register

================================================================================
MSG: firmware_common/MotorVersion
uint8 major
uint8 minor
uint8 patch

================================================================================
MSG: firmware_common/StallDetector1Feedback
# The velocity is filtered before used for stall detection. Telemetry the filtered velocity here for debugging purpose
float32[1] filtered_velocities
# The last result from the stall detection
uint8[1] stalled

================================================================================
MSG: firmware_common/TrajectoryGeneratorFeedback
uint32 num_remaining_setpoints

================================================================================
MSG: mobility_interface/LatchTelemetry
# Output velocity of the DC motor
firmware_common/DCMotorGroupFeedback dc_motor_group

================================================================================
MSG: firmware_common/DCMotorGroupFeedback
# TODO: dc motor group is almost useless, because most of the dc motors act on its own. Remove this later.
# Feedback from DC motors (controlled by a PWM signal and a direction pin)
DCMotorFeedback[1] motors
# Number of DC motors
uint8 motors_count

================================================================================
MSG: firmware_common/DCMotorFeedback
# The percent of last command velocity over max absolute velocity. The range is [-100, 100].
int8 vel_percent

================================================================================
MSG: mobility_interface/LiftV4Telemetry
# Lift motor's position, velocity and torque feedback
firmware_common/MotorGroup1Feedback motor_group
# State machine's state and mask
firmware_common/SingleMotorFSMFeedback lift_fsm
# Last command sent out to the motor
firmware_common/CommandRecorder1Feedback command_recorder
# Scaled and offseted motor position i.e. the joint space position
firmware_common/ConverterResult1Feedback joint_group
# Remaining points in the trajectory generator
firmware_common/TrajectoryGeneratorFeedback traj_generator
# Stall detection feedback, including the filtered velocity and the detection result
firmware_common/StallDetector1Feedback stall_detector
# Calibration result
firmware_common/Calibration1Feedback calibration_manager
# Brake status
firmware_common/BrakeGroupFeedback brake_group
# Airlock's pressure
firmware_common/AirLockSensorFeedback air_lock_sensor

================================================================================
MSG: firmware_common/AirLockSensorFeedback
uint16 voltage_reading_raw

================================================================================
MSG: firmware_common/SingleMotorFSMFeedback
uint8 state

# Transient state: request version information from the motors
uint8 INIT=0
# Transient state: send disable motor commands and wait for valid feedback
uint8 DISABLING=1
# Steady state: request feedback from the motors
uint8 DISABLED=2
# Transient state: send zero motor commands and wait for valid feedback
uint8 ZEROING=3
# Transient state: send enable motor commands and wait for valid feedback
uint8 ENABLING=4
# Transient state: send position gain motor commands and wait for valid feedback
uint8 TO_POS_MODE=5
# Steady state: request feedback from the motors. Also send "remove feedforward torque" if the brake is fully engaged
uint8 IDLE=6
# Transient state: wait for a valid trajectory
uint8 POS_PRE_MOVE=7
# Transient state: send setpoint commands to the motors until the whole trajectory is consumed
uint8 POS_MOVING=8
# Transient state: waiting for the motor to stall
uint8 POS_STALLING=9
# Steady state: arm already collided. KEEP SENDING DISABLE!
uint8 COLLIDED=10
# Transient state: sending velocity gain motor command and wait for valid feedback
uint8 TO_VEL_MODE=11
# Transient state: waiting for a valid trajectory
uint8 PRE_CALIBRATE=12
# Transient state: send setpoint commands to the motors until stall detected
uint8 CALIBRATING=13
# Steady state: motor version is incorrect. DO NOT SEND ANY CAN FRAME!
uint8 VERSION_ERROR=254
# Steady state: motor is in fatal error (usually CAN bus error). KEEP SENDING DISABLE!
uint8 FATAL_ERROR=255

uint8 TRAJ_TYPE_NONE=0
uint8 TRAJ_TYPE_BUFFER=1
uint8 TRAJ_TYPE_GENERATOR=2

================================================================================
MSG: mobility_interface/PivotTelemetry
# Pivot motor's position, velocity and torque feedback
firmware_common/MotorGroup1Feedback motor_group
# State machine's state and mask
firmware_common/SingleMotorFSMFeedback pivot_fsm
# Last command sent out to the motor
firmware_common/CommandRecorder1Feedback command_recorder
# Scaled and offseted motor position i.e. the joint space position
firmware_common/ConverterResult1Feedback joint_group
# Remaining points in the trajectory generator
firmware_common/TrajectoryGeneratorFeedback traj_generator
# Stall detection feedback, including the filtered velocity and the detection result
firmware_common/StallDetector1Feedback stall_detector
# Calibration result
firmware_common/Calibration1Feedback calibration_manager

================================================================================
MSG: mobility_interface/TractionTelemetry
# odometry node feedback
firmware_common/OdometryFeedback odom
# traction component feedback
firmware_common/TractionFeedback traction_feedback
# traction motor group's states
firmware_common/MotorGroup4Feedback motor_group
# grid induction sensor feedback states
firmware_common/GridSensorFeedback grid_sensor
# Last command sent out to the motor
firmware_common/CommandRecorder4Feedback command_recorder
# Stall detection
firmware_common/StallDetector4Feedback stall_detector
# traction's collision detector's feedback
firmware_common/TractionCollisionFeedback collision_feedback
firmware_common/TractionTrajFeedback trajectory
================================================================================
MSG: firmware_common/CommandRecorder4Feedback
LastCommand[4] last_commands

================================================================================
MSG: firmware_common/GridSensorFeedback
# relays the [high/low] state of the grid induction sensors
# the indicies and location map is described in qoowa/message/odometry.hpp
uint8 neg_x
uint8 pos_x
uint8 neg_y
uint8 pos_y
================================================================================
MSG: firmware_common/MotorGroup4Feedback
MotorFeedback[4] motor_feedbacks
uint8 master_error

================================================================================
MSG: firmware_common/OdometryFeedback
# enum value for current base motion direction and for component state, see qoowa/message/odometry.hpp for enum definition
uint8 state
uint8 base_motion_direction

# relative position on grid from start
int32 x_cell_count
int32 y_cell_count

# base distance (rad), speed (rad/s) and torque (Nm)
float32 combined_base_distance
float32 combined_base_vel
float32 combined_base_torque

# state to indicate that grid sensor in the direction of motion has gone HIGH
bool rising_edge_trigger
# state to indicate that the robot has travelled approximately one grid rail width after the rising edge
bool travel_grid_rail_width
# distance the robot has travelled since the trailing sensor has indicated a high value
float32 dist_since_last_grid_high

# odom node state enum definitions for correspondence
uint8 ODOMETRY_NODE_COMPONENT_STATE_ALL_WORKING = 0
uint8 ODOMETRY_NODE_COMPONENT_STATE_MOTOR_FAILED = 1
uint8 ODOMETRY_NODE_COMPONENT_STATE_GRID_SENSOR_FAILED = 2
uint8 ODOMETRY_NODE_COMPONENT_STATE_ALL_FAILED = 3

# bitwise for the component state
uint8 ODOMETRY_NODE_COMPONENT_STATE_MOTOR_BIT = 0
uint8 ODOMETRY_NODE_COMPONENT_STATE_GRID_SENSOR_BIT = 1

# odom node state enum definitions for correspondence
uint8 ODOMETRY_BASE_MOVING_DIRECTION_INVALID = 0
uint8 ODOMETRY_BASE_MOVING_DIRECTION_POSITIVE_X = 1
uint8 ODOMETRY_BASE_MOVING_DIRECTION_NEGATIVE_X = 2
uint8 ODOMETRY_BASE_MOVING_DIRECTION_POSITIVE_Y = 3
uint8 ODOMETRY_BASE_MOVING_DIRECTION_NEGATIVE_Y = 4

================================================================================
MSG: firmware_common/StallDetector4Feedback
# The velocity is filtered before used for stall detection. Telemetry the filtered velocity here for debugging purpose
float32[4] filtered_velocities
# The last result from the stall detection
uint8[4] stalled

================================================================================
MSG: firmware_common/TractionCollisionFeedback
# state to describe that collision has been detected [float(bool)]
uint8 collision_detected
# collision detection monitor state
uint8 monitor_state

# monitor states
# defined in detail qoowa/message/traction_collision_detection.hpp
uint8 MONITOR_STATE_NOT_MONITORING=0
uint8 MONITOR_STATE_SUPERVISING_TRACTION_COMPONENT=1
uint8 MONITOR_STATE_OBSERVING_HIGH_TORQUE=2
uint8 MONITOR_STATE_OBSERVING_LARGE_TRACKING_ERROR=3
uint8 MONITOR_STATE_COLLISION_IMMINENT=4
uint8 MONITOR_STATE_COLLISION_DETECTED=5
================================================================================
MSG: firmware_common/TractionFeedback
# traction state
uint8 state

uint8 TRACTION_FSM_STATE_INIT = 0
uint8 TRACTION_FSM_STATE_DISABLING = 1
uint8 TRACTION_FSM_STATE_DISABLED = 2
uint8 TRACTION_FSM_STATE_ZEROING = 3
uint8 TRACTION_FSM_STATE_ENABLING = 4
uint8 TRACTION_FSM_STATE_TO_POS_MODE = 5
uint8 TRACTION_FSM_STATE_POS_STALLING = 6
uint8 TRACTION_FSM_STATE_IDLE = 7
uint8 TRACTION_FSM_STATE_TO_VEL_MODE = 8
uint8 TRACTION_FSM_STATE_PRE_VEL_MOVE = 9
uint8 TRACTION_FSM_STATE_VEL_MOVING = 10
uint8 TRACTION_FSM_STATE_EMERGENCY_STOP = 11
uint8 TRACTION_FSM_STATE_VERSION_ERROR = 254
uint8 TRACTION_FSM_STATE_FATAL_ERROR = 255

================================================================================
MSG: firmware_common/TractionTrajFeedback
# sanity check feedback
uint8 travel_envelope

# tota distance commanded
float32 commanded_dist
# distance travelled so far
float32 dist_travelled
# distance travelled until the last cell was crossed
float32 distance_travelled_this_cell
# fixed distance travelled until last state reset
float32 distance_traveled_until_last_cell

# absolute value of command velocity
float32 abs_commanded_vel
# current state of alignment sequence
uint8 alignment_state
# Standard deviation of velocity over a moving window
float32[4] velocity_stddev

# current phase of trajectory
uint8 phase

# traction component phase definitions
uint8 TRACTION_PHASE_NONE = 0
uint8 TRACTION_PHASE_INIT = 1
uint8 TRACTION_PHASE_HOLDING_POSITION = 2
uint8 TRACTION_PHASE_FOLLOWING_TRAJ = 3
uint8 TRACTION_PHASE_ALIGNING_TO_GRID = 4
uint8 TRACTION_PHASE_EMERGENCY_DECELERATION = 5
uint8 TRACTION_PHASE_ALIGNMENT_FAILED = 6


# traction alignment state definitions
uint8 TRACTION_ALIGNMENT_STATE_NOT_MONITORING = 0
uint8 TRACTION_ALIGNMENT_STATE_WAITING_FOR_TRAIL_HIGH = 1
uint8 TRACTION_ALIGNMENT_STATE_WAITING_FOR_LEAD_HIGH = 2
uint8 TRACTION_ALIGNMENT_STATE_ALIGNMENT_FOUND = 3
"#;
        let mut type_table = HashMap::new();
        let result = parse(schema_name, schema_text, &mut type_table);
        result.unwrap();
    }
}
