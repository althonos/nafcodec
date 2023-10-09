import os
from typing import Union, Iterator, Optional, BytesIO

__version__: str
__author__: str

class Record:
    id: Optional[str]
    comment: Optional[str]
    sequence: Optional[str]
    quality: Optional[str]
    length: Optional[int]

class Decoder(Iterator[Record]):
    def __init__(self, file: Union[os.PathLike[str], BytesIO]) -> None: ...
    def __iter__(self) -> Decoder: ...
    def __next__(self) -> Record: ...