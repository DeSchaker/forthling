"""Tests of the Python module; run with `python -m unittest discover python/tests`."""

import json
import math
import unittest

import forthling


class InterpretTest(unittest.TestCase):
    def test_returns_the_output(self):
        f = forthling.Forth()
        self.assertEqual(f.interpret(": SQUARE DUP * ; 7 SQUARE ."), "49 ")
        self.assertEqual(f.interpret("2 SQUARE"), "")
        self.assertEqual(f.stack, [4])

    def test_stacks(self):
        f = forthling.Forth()
        f.push(6)
        f.push(7)
        f.interpret("*")
        self.assertEqual(f.pop(), 42)
        f.fpush(2.0)
        f.interpret("FSQRT")
        self.assertAlmostEqual(f.fpop(), math.sqrt(2))

    def test_errors_carry_code_and_output(self):
        f = forthling.Forth()
        with self.assertRaises(forthling.ForthError) as cm:
            f.interpret('.( before) 1 2 NOSUCHWORD')
        self.assertEqual(cm.exception.code, -13)
        self.assertEqual(cm.exception.output, "before")
        self.assertIn("undefined word: NOSUCHWORD", str(cm.exception))
        self.assertEqual(f.stack, [])
        self.assertEqual(cm.exception.stack, [1, 2])
        self.assertEqual(cm.exception.token, "NOSUCHWORD")
        self.assertEqual((cm.exception.line, cm.exception.column), (1, 16))
        f.interpret(": STEP 0 / ;\n: MAIN 1 2 STEP ;")
        with self.assertRaises(forthling.ForthError) as cm:
            f.interpret("5 MAIN")
        self.assertEqual(cm.exception.words, ["MAIN", "STEP", "/"])
        self.assertEqual(cm.exception.stack, [5, 1])
        with self.assertRaises(forthling.ForthError) as cm:
            f.pop()
        self.assertEqual(cm.exception.code, -4)
        self.assertIsNone(cm.exception.stack)

    def test_input_and_bye(self):
        f = forthling.Forth(input="A")
        self.assertEqual(f.interpret("KEY ."), "65 ")
        self.assertFalse(f.finished)
        f.interpret("BYE")
        self.assertTrue(f.finished)

    def test_memory_size(self):
        self.assertEqual(forthling.Forth(memory=1 << 16).interpret("1 ."), "1 ")
        with self.assertRaises(ValueError):
            forthling.Forth(memory=100)

    def test_environment(self):
        f = forthling.Forth()
        self.assertEqual(f.interpret('S" FORTHLING-PYTHON" ENVIRONMENT? . .'), "-1 1 ")

    def test_json(self):
        f = forthling.Forth()
        f.interpret("ALSO JSON")
        self.assertEqual(f.interpret('S" [1, 2, 3]" JSON-LENGTH .'), "3 ")
        out = f.interpret('JSON{ S" sum" JSON-KEY 6 JSON-NUMBER JSON}')
        self.assertEqual(json.loads(out), {"sum": 6})


class SandboxTest(unittest.TestCase):
    def test_limits(self):
        f = forthling.Forth(max_steps=10_000)
        with self.assertRaises(forthling.ForthError) as cm:
            f.interpret(": FOREVER BEGIN AGAIN ; FOREVER")
        self.assertEqual(cm.exception.code, -258)
        self.assertIn("steps", str(cm.exception))
        f = forthling.Forth(max_time=0.05)
        with self.assertRaises(forthling.ForthError) as cm:
            f.interpret(": FOREVER BEGIN AGAIN ; FOREVER")
        self.assertIn("time", str(cm.exception))
        f = forthling.Forth(max_output=5)
        with self.assertRaises(forthling.ForthError) as cm:
            f.interpret(": STARS 10 0 DO 42 EMIT LOOP ; STARS")
        self.assertEqual(cm.exception.output, "*****")
        with self.assertRaises(ValueError):
            forthling.Forth(max_time=-1)
        f = forthling.Forth(max_space=10_000)
        with self.assertRaises(forthling.ForthError) as cm:
            f.interpret(': D S" : X ;" EVALUATE ; : M 10000 0 DO D LOOP ; M')
        self.assertIn("space", str(cm.exception))

    def test_sandbox(self):
        f = forthling.Forth(sandbox=True)
        self.assertEqual(f.interpret('S" x.txt" R/O OPEN-FILE . .'), "-259 0 ")
        with self.assertRaises(forthling.ForthError) as cm:
            f.interpret("INCLUDE x.fs")
        self.assertEqual(cm.exception.code, -259)
        with self.assertRaises(forthling.ForthError) as cm:
            f.interpret(": FOREVER BEGIN AGAIN ; FOREVER")
        self.assertEqual(cm.exception.code, -258)
        self.assertEqual(f.interpret("6 7 * ."), "42 ")


class AddWordTest(unittest.TestCase):
    def test_arguments_and_results(self):
        f = forthling.Forth()
        f.add_word("HYPOT", math.hypot, inputs=2)
        f.add_word("DIVMOD", divmod, inputs=2)
        f.add_word("POSITIVE?", lambda n: n > 0, inputs=1)
        f.add_word("NOTHING", lambda: None)
        self.assertEqual(f.interpret("3 4 HYPOT F. 17 5 DIVMOD . ."), "5. 2 3 ")
        self.assertEqual(f.interpret("5 POSITIVE? . -5 POSITIVE? . NOTHING DEPTH ."), "-1 0 0 ")

    def test_words_work_in_definitions(self):
        f = forthling.Forth()
        seen = []
        f.add_word("LOG", seen.append, inputs=1)
        f.interpret(": LOGALL 3 0 DO I LOG LOOP ; LOGALL")
        self.assertEqual(seen, [0, 1, 2])

    def test_own_word_list(self):
        f = forthling.Forth()
        f.add_word("TWICE", lambda n: 2 * n, inputs=1, wordlist="PY")
        with self.assertRaises(forthling.ForthError):
            f.interpret("1 TWICE")
        self.assertEqual(f.interpret("ALSO PY 21 TWICE ."), "42 ")

    def test_python_exceptions(self):
        f = forthling.Forth()
        f.add_word("BOOM", lambda: 1 / 0)
        with self.assertRaises(ZeroDivisionError):
            f.interpret("BOOM")
        self.assertEqual(f.interpret("' BOOM CATCH ."), "-257 ")
        f.add_word("BAD", lambda: "text")
        with self.assertRaises(TypeError):
            f.interpret("BAD")
        f.add_word("BIG", lambda: 2**64)
        with self.assertRaises(OverflowError):
            f.interpret("BIG")

    def test_stack_underflow_in_a_word(self):
        f = forthling.Forth()
        f.add_word("ADD", lambda a, b: a + b, inputs=2)
        with self.assertRaises(forthling.ForthError) as cm:
            f.interpret("1 ADD")
        self.assertEqual(cm.exception.code, -4)

    def test_reentry_is_refused(self):
        f = forthling.Forth()
        f.add_word("AGAIN", lambda: f.interpret("1"))
        with self.assertRaises(RuntimeError):
            f.interpret("AGAIN")


if __name__ == "__main__":
    unittest.main()
