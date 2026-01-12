# Test fixture: Thread-safe Queue usage
# Expected: 0 race warnings (Queue is thread-safe)
import queue
import threading


class Worker:
    def __init__(self):
        self.queue = queue.Queue()  # Thread-safe

    def start(self):
        threading.Thread(target=self._process).start()

    def _process(self):
        while True:
            item = self.queue.get()  # Should NOT warn
            self.queue.task_done()


if __name__ == "__main__":
    w = Worker()
    w.start()
