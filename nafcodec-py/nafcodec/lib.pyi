import os
from typing import Union, Iterator, Optional, BinaryIO

__version__: str
__author__: str

class Record:
    id: Optional[str]
    comment: Optional[str]
    sequence: Optional[str]
    quality: Optional[str]
    length: Optional[int]
    def __init__(
        self,
        *,
        id: Optional[str] = None,
        comment: Optional[str] = None,
        sequence: Optional[str] = None,
        quality: Optional[str] = None,
        length: Optional[int] = None
    ): ...
    def __repr__(self) -> str: ...

class Decoder(Iterator[Record]):
    def __init__(self, file: Union[os.PathLike[str], BinaryIO]) -> None: ...
    def __iter__(self) -> Decoder: ...
    def __next__(self) -> Record: ...
