import gzip
import io
import os
import tempfile
import unittest

import nafcodec
from . import data

try:
    try:
        from importlib.resources import files
    except ImportError:
        from importlib_resources import files  # type: ignore
except ImportError:
    files = None  # type: ignore


class TestEncoder(unittest.TestCase):

    def test_invalid_sequence(self):
        buffer = io.BytesIO()
        with self.assertRaises(ValueError):
            encoder = nafcodec.Encoder(buffer, sequence_type="dna", sequence=True)
            encoder.write(nafcodec.Record(sequence="hello world?!"))

    def test_missing_field(self):
        buffer = io.BytesIO()
        with self.assertRaises(ValueError):
            encoder = nafcodec.Encoder(buffer, sequence_type="dna", sequence=True)
            encoder.write(nafcodec.Record())
        with self.assertRaises(ValueError):
            encoder = nafcodec.Encoder(buffer, sequence_type="dna", sequence=True)
            encoder.write(nafcodec.Record(id="r1"))

    def test_dna_records(self):
        buffer = io.BytesIO()

        with nafcodec.Encoder(buffer, sequence_type="dna", id=True, sequence=True) as f:
            f.write(nafcodec.Record(id="r1", sequence="ATTATTAGACAGAGC"))
            f.write(nafcodec.Record(id="r2", sequence="CTATTG"))
            f.write(nafcodec.Record(id="r3", sequence="TTAGTNNNNN"))

        buffer.seek(0)
        decoder = nafcodec.Decoder(buffer)
        records = list(decoder)

        self.assertEqual(len(records), 3)
        self.assertEqual(records[0].id, "r1")
        self.assertEqual(records[1].id, "r2")
        self.assertEqual(records[2].id, "r3")
        self.assertEqual(records[0].sequence, "ATTATTAGACAGAGC")
        self.assertEqual(records[1].sequence, "CTATTG")
        self.assertEqual(records[2].sequence, "TTAGTNNNNN")

        for i in range(3):
            self.assertIs(records[i].quality, None)
            self.assertIs(records[i].comment, None)

    def test_fastq_records(self):
        buffer = io.BytesIO()

        with nafcodec.Encoder(
            buffer, sequence_type="rna", id=True, sequence=True, quality=True
        ) as f:
            f.write(nafcodec.Record(id="r1", sequence="AUUAU", quality="GGGGG"))
            f.write(nafcodec.Record(id="r2", sequence="CUAUU", quality="#8A@C"))
            f.write(nafcodec.Record(id="r3", sequence="UUAGU", quality="CCGGG"))

        buffer.seek(0)
        decoder = nafcodec.Decoder(buffer)
        records = list(decoder)

        self.assertEqual(len(records), 3)
        self.assertEqual(records[0].id, "r1")
        self.assertEqual(records[1].id, "r2")
        self.assertEqual(records[2].id, "r3")
        self.assertEqual(records[0].sequence, "AUUAU")
        self.assertEqual(records[1].sequence, "CUAUU")
        self.assertEqual(records[2].sequence, "UUAGU")
        self.assertEqual(records[0].quality, "GGGGG")
        self.assertEqual(records[1].quality, "#8A@C")
        self.assertEqual(records[2].quality, "CCGGG")

        for i in range(3):
            self.assertIs(records[i].comment, None)
