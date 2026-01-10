# Test fixture: Thread-safe deque usage
# Expected: 0 race warnings (deque append/pop are atomic)
import threading
from collections import deque


class TaskQueue:
    def __init__(self):
        self.tasks = deque()  # Thread-safe for append/pop

    def add_task(self, task):
        self.tasks.append(task)

    def start_worker(self):
        threading.Thread(target=self._worker).start()

    def _worker(self):
        while True:
            try:
                task = self.tasks.popleft()  # Atomic operation
                task()
            except IndexError:
                pass  # Empty queue


if __name__ == "__main__":
    q = TaskQueue()
    q.start_worker()
