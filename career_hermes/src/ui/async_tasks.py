"""
Qt helpers for running async service calls without blocking the UI thread.
"""

from __future__ import annotations

import asyncio
from typing import Any, Awaitable, Callable, Iterable, Optional

from PySide6.QtCore import QObject, QRunnable, QThreadPool, Signal, Slot
from PySide6.QtWidgets import QLabel, QWidget


class AsyncTaskSignals(QObject):
    """Signals emitted by a background async task."""

    succeeded = Signal(object)
    failed = Signal(object)
    finished = Signal()


class AsyncTask(QRunnable):
    """Run a coroutine factory in a thread-pool worker."""

    def __init__(self, coro_factory: Callable[[], Awaitable[Any]]) -> None:
        super().__init__()
        self.coro_factory = coro_factory
        self.signals = AsyncTaskSignals()

    @Slot()
    def run(self) -> None:
        try:
            result = asyncio.run(self.coro_factory())
        except Exception as exc:
            self.signals.failed.emit(exc)
        else:
            self.signals.succeeded.emit(result)
        finally:
            self.signals.finished.emit()


def start_async_task(
    owner: QWidget,
    status_label: QLabel,
    message: str,
    coro_factory: Callable[[], Awaitable[Any]],
    on_success: Callable[[Any], None],
    on_error: Optional[Callable[[Exception], None]] = None,
    busy_widgets: Iterable[QWidget] = (),
) -> AsyncTask:
    """
    Start an async task on the global QThreadPool and keep UI state coherent.

    The owner keeps a reference to active tasks so PySide does not collect the
    QRunnable wrapper before its signals are delivered.
    """

    task = AsyncTask(coro_factory)
    tasks = getattr(owner, "_async_tasks", None)
    if tasks is None:
        tasks = set()
        setattr(owner, "_async_tasks", tasks)
    tasks.add(task)

    widgets = list(busy_widgets)
    enabled_states = {widget: widget.isEnabled() for widget in widgets}
    for widget in widgets:
        widget.setEnabled(False)
    status_label.setText(message)

    def restore_widgets() -> None:
        for widget in widgets:
            widget.setEnabled(enabled_states.get(widget, True))

    def handle_success(result: Any) -> None:
        restore_widgets()
        on_success(result)

    def handle_error(exc: Exception) -> None:
        restore_widgets()
        if on_error is not None:
            on_error(exc)

    def handle_finished() -> None:
        status_label.clear()
        tasks.discard(task)

    task.signals.succeeded.connect(handle_success)
    task.signals.failed.connect(handle_error)
    task.signals.finished.connect(handle_finished)
    QThreadPool.globalInstance().start(task)
    return task
