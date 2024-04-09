import os
import typing
from types import TracebackType
from typing import Type, Union, Iterator, Optional, BinaryIO, ContextManager

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
        length: Optional[int] = None,
    ): ...
    def __repr__(self) -> str: ...

class Decoder(Iterator[Record], ContextManager[Decoder]):
    def __init__(
        self,
        file: Union[str, os.PathLike[str], BinaryIO],
        *,
        id: bool = True,
        comment: bool = True,
        sequence: bool = True,
        quality: bool = True,
        mask: bool = True,
        buffer_size: Optional[int] = None,
    ) -> None: ...
    def __iter__(self) -> Decoder: ...
    def __next__(self) -> Record: ...
    def __len__(self) -> int: ...
    def __enter__(self) -> Decoder: ...
    def __exit__(
        self,
        exc_type: Optional[Type[BaseException]],
        exc_value: Optional[BaseException],
        traceback: Optional[TracebackType],
    ) -> bool: ...
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
    def read(self) -> Optional[Record]: ...

class Encoder(ContextManager[Encoder]):
    def __init__(
        self,
        file: Union[str, os.PathLike[str], BinaryIO],
        sequence_type: SEQUENCE_TYPE = "dna",
        *,
        id: bool = False,
        comment: bool = False,
        sequence: bool = False,
        quality: bool = False,
    ): ...
    def __enter__(self) -> Encoder: ...
    def __exit__(
        self,
        exc_type: Optional[Type[BaseException]],
        exc_value: Optional[BaseException],
        traceback: Optional[TracebackType],
    ) -> bool: ...
    def write(self, record: Record) -> None: ...
    def close(self) -> None: ...

@typing.overload
def open(
    file: Union[str, os.PathLike[str], BinaryIO],
    mode: Literal["r"],
    **options,
) -> Decoder: ...
@typing.overload
def open(
    file: Union[str, os.PathLike[str], BinaryIO],
    mode: Literal["w"],
    sequence_type: SEQUENCE_TYPE = "dna",
    **options,
) -> Encoder: ...
@typing.overload
def open(
    file: Union[str, os.PathLike[str], BinaryIO],
    mode: Literal["r", "w"] = "r",
    **options,
) -> Union[Decoder, Encoder]: ...
