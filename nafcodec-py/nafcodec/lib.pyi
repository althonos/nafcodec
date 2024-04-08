import os
import typing
from typing import Union, Iterator, Optional, BinaryIO

try:
    from typing import Literal
except ImportError:
    from typing_extensions import Literal  # type: ignore

__version__: str
__author__: str

if typing.TYPE_CHECKING:
    SEQUENCE_TYPE = Literal["dna", "rna", "protein", "text"]
    FORMAT_VERSION = Literal["v1", "v2"]

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
    def __init__(self, file: Union[str, os.PathLike[str], BinaryIO]) -> None: ...
    def __iter__(self) -> Decoder: ...
    def __next__(self) -> Record: ...
    @property
    def sequence_type(self) -> SEQUENCE_TYPE: ...
    @property
    def format_version(self) -> FORMAT_VERSION: ...
    @property
    def line_length(self) -> int: ...
    @property
    def name_separator(self) -> str: ...
    @property
    def number_of_sequences(self) -> int: ...