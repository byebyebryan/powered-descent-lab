"""Final reward revision: discount-aligned shaping with zero terminal potential."""
from pd_env_v2 import CONFIGURATION as BRAKING_CONFIGURATION
from pd_env_v2 import ProceduralLandingEnv as BrakingEnvironment

CONFIGURATION = {
    **BRAKING_CONFIGURATION,
    "reward": "discount-aligned-terminal-potential-v3",
    "potential_discount": 0.999,
    "potential_discount_clock": "policy-decision",
    "terminal_potential": 0.0,
}


def potential_reward(previous, following, terminal):
    return 100.0*((0.0 if terminal else CONFIGURATION["potential_discount"]*following)-previous)


class ProceduralLandingEnv(BrakingEnvironment):
    def __init__(self, *args, **kwargs):
        kwargs.setdefault("agent_id", "ppo-terrain-v3")
        super().__init__(*args, **kwargs)

    def _reward_components(self, old, physical, mission, done):
        previous = self.potential
        parts = super()._reward_components(old, physical, mission, done)
        parts["potential"] = potential_reward(previous, self.potential, done)
        if done:
            self.potential = 0.0
        return parts
