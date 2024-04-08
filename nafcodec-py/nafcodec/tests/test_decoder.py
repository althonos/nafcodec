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


class _TestDecoder(object):

    def _get_decoder(self, filename):
        raise NotImplementedError

    @unittest.skipUnless(files, "importlib.resources not found")
    def test_fastq(self):
        decoder = self._get_decoder("phix.naf")
        self.assertEqual(decoder.sequence_type, "dna")
        records = list(decoder)
        self.assertEqual(len(records), 42)
        self.assertEqual(records[0].id, "SRR1377138.1")
        self.assertEqual(records[0].sequence[:36], "NGCTCTTAAACCTGCTATTGAGGCTTGTGGCATTTC")
        self.assertEqual(records[0].quality[:31], "#8CCCGGGGGGGGGGGGGGGGGGGGGGGGGG")

    @unittest.skipUnless(files, "importlib.resources not found")
    def test_dna(self):
        decoder = self._get_decoder("CP040672.naf")
        self.assertEqual(decoder.sequence_type, "dna")
        records = list(decoder)
        self.assertEqual(len(records), 100)
        self.assertEqual(records[0].id, "lcl|NZ_CP040672.1_cds_WP_044801954.1_1")
        self.assertEqual(records[0].sequence.count('A'), 181)
        self.assertEqual(records[0].sequence.count('C'), 200)
        self.assertEqual(records[0].sequence.count('G'), 210)
        self.assertEqual(records[0].sequence.count('T'), 240)
        self.assertIs(records[0].quality, None)

    @unittest.skipUnless(files, "importlib.resources not found")
    def test_protein(self):
        decoder = self._get_decoder("LuxC.naf")
        self.assertEqual(decoder.sequence_type, "protein")
        records = list(decoder)
        self.assertEqual(len(records), 12)
        self.assertEqual(records[0].id, "sp|P19841|LUXC_PHOPO")
        self.assertEqual(records[0].sequence[:25], "MCNAEFKGDCMIKKIPMIIGGAERD")
        self.assertIs(records[0].quality, None)
        self.assertEqual(records[5].id, "sp|P29236|LUXC2_PHOLE")
        self.assertEqual(records[5].sequence[:25], "MIKKIPMIIGGVVQNTSGYGMRELT")
        self.assertIs(records[5].quality, None)

    @unittest.skipUnless(files, "importlib.resources not found")
    def test_dna_masked(self):
        decoder = self._get_decoder("masked.naf")
        self.assertEqual(decoder.sequence_type, "dna")
        records = list(decoder)
        self.assertEqual(len(records), 2)
        self.assertEqual(records[0].id, "test1")
        self.assertTrue(records[0].sequence[:657].isupper())
        self.assertTrue(records[0].sequence[657:676].islower())
        self.assertTrue(records[0].sequence[676:1311].isupper())
        self.assertTrue(records[0].sequence[1311:1350].islower())
        self.assertIs(records[0].quality, None)


class TestDecoderHandle(_TestDecoder, unittest.TestCase):

    def _get_decoder(self, filename):
        with files(data).joinpath(filename).open("rb") as f:
            content = f.read()
        handle = io.BytesIO(content)
        return nafcodec.Decoder(handle)


class TestDecoderFile(_TestDecoder, unittest.TestCase):
    
    def setUp(self):
        self.handle = None

    def tearDown(self):
        if self.handle is not None:
            self.handle.close()

    def _get_decoder(self, filename):
        self.handle = files(data).joinpath(filename).open("rb")
        return nafcodec.Decoder(self.handle)

    def test_error_filenotfound(self):
        with self.assertRaises(FileNotFoundError):
            decoder = nafcodec.Decoder("")
        
    @unittest.skipIf(os.name == "nt", "Windows error codes differ")
    def test_error_isadirectory(self):
        with self.assertRaises(IsADirectoryError):
            decoder = nafcodec.Decoder(os.path.dirname(__file__))