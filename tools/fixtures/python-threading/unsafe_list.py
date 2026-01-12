# Test fixture: Unsafe list usage with threading
# Expected: 1 race warning (list is NOT thread-safe)
import threading


class Worker:
    def __init__(self):
        self.items = []  # NOT thread-safe

    def start(self):
        threading.Thread(target=self._process).start()

    def _process(self):
        self.items.append(1)  # SHOULD warn - list not safe


if __name__ == "__main__":
    w = Worker()
    w.start()
