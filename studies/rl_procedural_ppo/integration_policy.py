"""Untrained deterministic policy for platform integration, with no optimizer."""
import numpy as np


class DeterministicIntegrationPolicy:
    def predict(self, observation, deterministic=True):
        # Uses the existing approach's action mapping and launch primitive.
        # A constant action exercises capture/report integration, not competence.
        return np.zeros(2, dtype=np.float32), None
