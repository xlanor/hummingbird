pub mod mariadb;
pub mod postgres;
pub mod sqlite;

use crate::domain::library::dao::LibraryDao;
use crate::domain::playlist::dao::PlaylistDao;
use crate::domain::scanner::dao::ScannerDao;
use crate::domain::user::dao::UserDao;

pub trait Database: LibraryDao + PlaylistDao + UserDao + ScannerDao {}
impl<T: LibraryDao + PlaylistDao + UserDao + ScannerDao> Database for T {}
