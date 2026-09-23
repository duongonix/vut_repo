export const hello =
  'fn main():\n  name: str = "World"\n  out("Hello, $name!")\n\n# Simple. Expressive. Vut.';

export const examples = [
  {
    label: 'Simple',
    code: '# A simple example in Vut\n\nfn greet(name: str):\n  out("Hello, $name!")\n\nfn main():\n  greet("Vut")\n\n# Clean. Readable. Powerful.'
  },
  {
    label: 'Typed',
    code: '# Clear types. Clear intentions.\n\nname: str = "Vut"\nage: int = 1\nactive: bool = true\n\n# Inference when you want it.\nmessage = "Hello, $name!"'
  },
  {
    label: 'Flexible',
    code: '# Structured data, naturally.\n\ndata Point:\n  x: int\n  y: int\n\norigin = Point(x: 0, y: 0)\n\n# Names make your intent clear.'
  },
  {
    label: 'Powerful',
    code: '# Less ceremony. More expression.\n\nfn square(value: int) -> int:\n  value * value\n\nfn main():\n  out("The answer is $(square(7))")\n\n# The final expression is the return value.'
  }
];
