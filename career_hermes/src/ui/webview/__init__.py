from typing import List

from .bridge import CareerBridge

__all__: List[str] = ['CareerBridge', 'CareerWebWindow']


def __getattr__(name: str) -> object:
    if name == "CareerWebWindow":
        from .webview_window import CareerWebWindow

        return CareerWebWindow
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
