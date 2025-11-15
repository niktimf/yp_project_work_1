pub mod csv;
pub mod binary;
pub mod txt;

use std::io::{Read, Write};
use crate::errors::Result;


/// Трейт для парсинга данных из потока
pub trait Parser<T> {    
    /// Парсит данные из reader в вектор записей типа T
    /// 
    /// # Arguments
    /// * `reader` - Источник данных, реализующий трейт Read
    /// 
    /// # Returns
    /// Возвращает вектор распарсенных записей
    fn parse<R: Read>(reader: R) -> Result<Vec<T>>;
}

/// Трейт для сериализации данных в поток
pub trait Serializer<T> {
    /// Сериализует вектор записей в writer
    /// 
    /// # Arguments
    /// * `data` - Слайс записей для сериализации
    /// * `writer` - Приемник данных, реализующий трейт Write
    /// 
    /// # Returns
    /// Возвращает Ok(()) при успешной сериализации или ошибку ParseError
    fn serialize<W: Write>(data: &[T], writer: W) -> Result<()>;
}

/// Трейт для полного формата данных (парсинг + сериализация)
pub trait Format<T>: Parser<T> + Serializer<T> {}
impl<T, F> Format<T> for F where F: Parser<T> + Serializer<T> {}

