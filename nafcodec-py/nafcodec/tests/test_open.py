import gzip
import io
import os
import tempfile
import unittest
import shutil

import nafcodec
from . import data

try:
    try:
        from importlib.resources import files
    except ImportError:
        from importlib_resources import files  # type: ignore
except ImportError:
    files = None  # type: ignore


class TestOpen(unittest.TestCase):

    @unittest.skipUnless(files, "importlib.resources not found")
    def test_open_read_fileobj(self):
        with files(data).joinpath("LuxC.naf").open("rb") as f:
            with nafcodec.open(f, "r") as decoder:
                self.assertIsInstance(decoder, nafcodec.Decoder)
                self.assertEqual(len(decoder), 12)
                self.assertEqual(len(list(decoder)), 12)

    @unittest.skipUnless(files, "importlib.resources not found")
    def test_open_read_filename(self):
        with tempfile.NamedTemporaryFile(suffix=".naf") as dst:
            with files(data).joinpath("LuxC.naf").open("rb") as f:
                shutil.copyfileobj(f, dst)
            dst.flush()
            with nafcodec.open(dst.name, "r") as decoder:
                self.assertIsInstance(decoder, nafcodec.Decoder)
                self.assertEqual(len(decoder), 12)
                self.assertEqual(len(list(decoder)), 12)

    def test_open_write_fileobj(self):
        buffer = io.BytesIO()
        with nafcodec.open(buffer, "w", id=True) as encoder:
            self.assertIsInstance(encoder, nafcodec.Encoder)
            encoder.write(nafcodec.Record(id="r1"))
            encoder.write(nafcodec.Record(id="r2"))
            encoder.write(nafcodec.Record(id="r3"))
        buffer.seek(0)
        with nafcodec.open(buffer, "r") as decoder:
            r = decoder.read()
            self.assertEqual(r.id, "r1")
            self.assertIs(r.sequence, None)
            self.assertIs(r.comment, None)
            self.assertIs(r.quality, None)
            self.assertIs(r.length, None)

    def test_open_write_filename(self):
        with tempfile.NamedTemporaryFile(suffix=".naf") as tmp:
            with nafcodec.open(tmp.name, "w", id=True) as encoder:
                self.assertIsInstance(encoder, nafcodec.Encoder)
                encoder.write(nafcodec.Record(id="r1"))
                encoder.write(nafcodec.Record(id="r2"))
                encoder.write(nafcodec.Record(id="r3"))
            with nafcodec.open(tmp.name, "r") as decoder:
                r = decoder.read()
                self.assertEqual(r.id, "r1")
                self.assertIs(r.sequence, None)
                self.assertIs(r.comment, None)
                self.assertIs(r.quality, None)
                self.assertIs(r.length, None)
