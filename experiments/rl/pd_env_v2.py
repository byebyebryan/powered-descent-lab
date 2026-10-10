"""Reward-only revision: make braking and stable contact visible during training."""
import math

from pd_env import CONFIGURATION as BASE_CONFIGURATION
from pd_env import ProceduralLandingEnv as BaseEnvironment

CONFIGURATION = {
    **BASE_CONFIGURATION,
    "reward": "kinetic-attitude-potential-v2",
    "kinetic_weight": 2.0,
    "kinetic_ratio_scale": 0.25,
    "braking_acceleration_reference_mps2": 3.0,
    "attitude_weight": 0.5,
    "attitude_clearance_scale_m": 30.0,
}


class ProceduralLandingEnv(BaseEnvironment):
    """Same complete missions, physical interface, features and action mapping.

    The shaping potential adds bounded kinetic and near-terrain attitude costs.
    It neither supplies a command nor queries an incumbent or planned trajectory.
    """

    def __init__(self, *args, **kwargs):
        kwargs.setdefault("agent_id", "ppo-terrain-v2")
        super().__init__(*args, **kwargs)

    def _potential(self, observation):
        o = observation
        vehicle = self.physical["vehicle"]
        distance = math.hypot(o[11], o[12]-self.base_offset)
        safe_speed_squared = (vehicle["safe_touchdown_normal_speed_mps"]**2
                              + vehicle["safe_touchdown_tangential_speed_mps"]**2)
        denominator = (2*CONFIGURATION["braking_acceleration_reference_mps2"]*distance
                       + safe_speed_squared)
        kinetic = CONFIGURATION["kinetic_weight"]*math.tanh(
            CONFIGURATION["kinetic_ratio_scale"]*(o[4]**2+o[5]**2)/denominator)
        angular_error = ((math.sin(o[6])/vehicle["safe_touchdown_attitude_error_rad"])**2
                         + (o[7]/vehicle["safe_touchdown_angular_rate_radps"])**2)
        attitude = (CONFIGURATION["attitude_weight"]
                    * math.exp(-max(o[16], 0)/CONFIGURATION["attitude_clearance_scale_m"])
                    * math.tanh(angular_error))
        return super()._potential(o)-kinetic-attitude
