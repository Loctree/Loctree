# Test fixture: Thread-safe multiprocessing.Queue usage
# Expected: 0 race warnings (multiprocessing.Queue is thread-safe)
import multiprocessing
import threading


class MultiWorker:
    def __init__(self):
        self.queue = multiprocessing.Queue()  # Thread-safe

    def start(self):
        threading.Thread(target=self._process).start()

    def _process(self):
        while True:
            item = self.queue.get()  # Should NOT warn
            print(f"Processing: {item}")


if __name__ == "__main__":
    w = MultiWorker()
    w.start()
