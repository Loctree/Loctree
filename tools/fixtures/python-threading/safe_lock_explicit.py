# Test fixture: Explicit Lock usage (already supported)
# Expected: 0 race warnings (Lock provides synchronization)
import threading


class Counter:
    def __init__(self):
        self.value = 0
        self.lock = threading.Lock()  # Explicit lock

    def increment(self):
        with self.lock:
            self.value += 1

    def start_workers(self, n):
        for _ in range(n):
            threading.Thread(target=self._worker).start()

    def _worker(self):
        for _ in range(1000):
            self.increment()


if __name__ == "__main__":
    c = Counter()
    c.start_workers(4)
