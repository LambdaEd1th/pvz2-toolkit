use std::io::{self, Write};

pub struct XmlWriter<W: Write> {
    writer: W,
    indent_level: usize,
    indent_str: String,
}

impl<W: Write> XmlWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            indent_level: 0,
            indent_str: "\t".to_string(),
        }
    }

    pub fn write_header(&mut self) -> io::Result<()> {
        // XFL often omits the standard <?xml ...?> declaration or uses a specific one.
        // For now, we'll write nothing as per Sen's behavior (it seems to omit it in some cases or write it manually).
        Ok(())
    }

    pub fn start_element(&mut self, name: &str, attrs: &[(&str, &str)]) -> io::Result<()> {
        self.write_indent()?;
        write!(self.writer, "<{}", name)?;
        for (key, value) in attrs {
            write!(self.writer, " {}=\"{}\"", key, value)?;
        }
        writeln!(self.writer, ">")?;
        self.indent_level += 1;
        Ok(())
    }

    pub fn end_element(&mut self, name: &str) -> io::Result<()> {
        self.indent_level -= 1;
        self.write_indent()?;
        writeln!(self.writer, "</{}>", name)?;
        Ok(())
    }

    pub fn write_element(
        &mut self,
        name: &str,
        attrs: &[(&str, &str)],
        content: Option<&str>,
    ) -> io::Result<()> {
        self.write_indent()?;
        write!(self.writer, "<{}", name)?;
        for (key, value) in attrs {
            write!(self.writer, " {}=\"{}\"", key, value)?;
        }

        if let Some(c) = content {
            write!(self.writer, ">")?;
            write!(self.writer, "{}", c)?;
            writeln!(self.writer, "</{}>", name)?;
        } else {
            writeln!(self.writer, " />")?;
        }
        Ok(())
    }

    // For manual writing of complex nested structures in a single line if needed
    pub fn write_raw(&mut self, content: &str) -> io::Result<()> {
        self.write_indent()?;
        writeln!(self.writer, "{}", content)?;
        Ok(())
    }

    fn write_indent(&mut self) -> io::Result<()> {
        for _ in 0..self.indent_level {
            write!(self.writer, "{}", self.indent_str)?;
        }
        Ok(())
    }
}
