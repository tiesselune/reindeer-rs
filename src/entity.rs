use std::{fs::File, io::ErrorKind};

use serde::{de::DeserializeOwned, Serialize};
use sled::{Db, IVec, Tree};
use std::convert::TryInto;

pub trait Entity: Serialize + DeserializeOwned {
    type Key: AsBytes;

    fn tree_name() -> &'static str;
    fn get_key(&self) -> Self::Key;

    fn get_tree(db: &Db) -> std::io::Result<Tree> {
        db.open_tree(Self::tree_name())
            .map_err(|_| std::io::Error::new(ErrorKind::Other, "Could not open tree"))
    }

    fn from_ivec(vec: IVec) -> Self {
        bincode::deserialize::<Self>(vec.as_ref()).unwrap()
    }

    fn to_ivec(&self) -> IVec {
        IVec::from(bincode::serialize(self).unwrap())
    }

    fn get(key: &Self::Key, db: &Db) -> std::io::Result<Option<Self>> {
        Self::get_from_u8_array(&key.as_bytes(), db)
    }

    fn get_number(db: &Db) -> std::io::Result<usize> {
        Ok(Self::get_tree(db)?.len())
    }

    fn get_from_u8_array(key: &[u8], db: &Db) -> std::io::Result<Option<Self>> {
        Ok(Self::get_tree(db)?
            .get(key)
            .map_err(|_| std::io::Error::new(ErrorKind::Other, "Could not search tree"))?
            .map(|vec| Self::from_ivec(vec)))
    }

    fn get_with_prefix(key: impl AsBytes, db: &Db) -> std::io::Result<Vec<Self>> {
        Ok(Self::get_tree(db)?
            .scan_prefix(key.as_bytes())
            .map(|elem| Self::from_ivec(elem.unwrap().1))
            .collect())
    }

    fn get_in_range(start: impl AsBytes, end: impl AsBytes, db: &Db) -> std::io::Result<Vec<Self>> {
        Ok(Self::get_tree(db)?
            .range(start.as_bytes()..end.as_bytes())
            .map(|elem| Self::from_ivec(elem.unwrap().1))
            .collect())
    }

    fn get_from_start(
        start: usize,
        offset: usize,
        prefix: Option<impl AsBytes>,
        db: &Db,
    ) -> std::io::Result<Vec<Self>> {
        let mut iter = if let Some(prefix) = prefix {
            Self::get_tree(db)?.scan_prefix(prefix.as_bytes())
        } else {
            Self::get_tree(db)?.iter()
        };
        let mut result = Vec::new();
        for i in 0..(start + offset) {
            match iter.next() {
                Some(e) => {
                    if i >= start {
                        result.push(Self::from_ivec(e.unwrap().1));
                    }
                }
                None => return Ok(result),
            }
        }
        Ok(result)
    }

    fn get_from_end(
        start: usize,
        offset: usize,
        prefix: Option<impl AsBytes>,
        db: &Db,
    ) -> std::io::Result<Vec<Self>> {
        let mut iter = if let Some(prefix) = prefix {
            Self::get_tree(db)?.scan_prefix(prefix.as_bytes())
        } else {
            Self::get_tree(db)?.iter()
        };
        let mut result = Vec::new();
        for i in 0..(start + offset) {
            match iter.next_back() {
                Some(e) => {
                    if i >= start {
                        result.push(Self::from_ivec(e.unwrap().1));
                    }
                }
                None => break,
            }
        }
        result.reverse();
        Ok(result)
    }

    fn get_with_filter<F: Fn(&Self) -> bool>(f: F, db: &Db) -> std::io::Result<Vec<Self>> {
        Ok(Self::get_tree(db)?
            .iter()
            .map(|elem| Self::from_ivec(elem.unwrap().1))
            .filter(|e| f(e))
            .collect())
    }

    fn get_all(db: &Db) -> std::io::Result<Vec<Self>> {
        Ok(Self::get_tree(db)?
            .iter()
            .map(|elem| Self::from_ivec(elem.unwrap().1))
            .collect())
    }

    fn get_each(keys: &[Self::Key], db: &Db) -> Vec<Self> {
        keys.iter()
            .map(|key| Self::get(key, db))
            .filter_map(|res| match res {
                Ok(opt) => opt,
                Err(_) => None,
            })
            .collect()
    }

    fn save(&self, db: &Db) -> std::io::Result<()> {
        Self::get_tree(db)?.insert(
            &self.get_key().as_bytes(),
            bincode::serialize(self).unwrap(),
        )?;
        Ok(())
    }

    fn update<F: Fn(&mut Self)>(key: &Self::Key, f: F, db: &Db) -> std::io::Result<()> {
        Self::get_tree(db)?
            .fetch_and_update(&key.as_bytes(), |e| {
                e.map(|u8_arr| {
                    let mut value: Self = Self::from_ivec(IVec::from(u8_arr));
                    f(&mut value);
                    value.to_ivec()
                })
            })
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::Other, "Could not update object")
            })?;
        Ok(())
    }

    fn remove(key: &Self::Key, db: &Db) -> std::io::Result<()> {
        Self::get_tree(db)?.remove(&key.as_bytes())?;
        Ok(())
    }

    fn remove_from_u8_array(key: &[u8], db: &Db) -> std::io::Result<()> {
        Self::get_tree(db)?.remove(key)?;
        Ok(())
    }

    fn remove_prefixed(prefix: impl AsBytes, db: &Db) -> std::io::Result<()> {
        for entity in Self::get_with_prefix(prefix, db)? {
            Self::remove(&entity.get_key(), db)?;
        }
        Ok(())
    }

    fn filter_remove<F: Fn(&Self) -> bool>(f: F, db: &Db) -> std::io::Result<Vec<Self>> {
        let res = Self::get_with_filter(f, db)?;
        for entity in &res {
            Self::remove(&entity.get_key(), db)?;
        }
        Ok(res)
    }

    fn filter_update<F: Fn(&Self) -> bool, M: Fn(&mut Self)>(
        filter: F,
        modifier: M,
        db: &Db,
    ) -> std::io::Result<()> {
        let mut res = Self::get_with_filter(filter, db)?;
        for entity in &mut res {
            modifier(entity);
            entity.save(db)?;
        }
        Ok(())
    }

    fn exists(key: &Self::Key, db: &Db) -> std::io::Result<bool> {
        Self::get_tree(db)?
            .contains_key(&key.as_bytes())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    }

    fn export_json(f: File, db: &Db) -> std::io::Result<()> {
        let all = Self::get_all(db)?;
        serde_json::to_writer(f, &all)
            .map_err(|_| std::io::Error::new(ErrorKind::Other, "Could not serialize objects"))
    }

    fn import_json(f: File, db: &Db) -> std::io::Result<()> {
        let all: Vec<Self> = serde_json::from_reader(f)
            .map_err(|_| std::io::Error::new(ErrorKind::Other, "Could not deserialize objects"))?;
        for each in all {
            each.save(db)?;
        }
        Ok(())
    }
}

pub trait SetKey: Entity {
    fn set_key(&mut self, key: &Self::Key);
}

pub trait IncrementableEntity: Entity<Key = u32> {
    fn get_next_key(db: &Db) -> std::io::Result<u32>;
    fn save_next(&mut self, db: &Db) -> std::io::Result<u32>;
}

impl<T> IncrementableEntity for T
where
    T: Entity<Key = u32> + SetKey,
{
    fn get_next_key(db: &Db) -> std::io::Result<u32> {
        match Self::get_tree(db)?.last()? {
            Some((key, _)) => Ok(u32::from_be_bytes(key.as_ref().try_into().unwrap()) + 1),
            None => Ok(Default::default()),
        }
    }

    fn save_next(&mut self, db: &Db) -> std::io::Result<u32> {
        let next_key = Self::get_next_key(db)?;
        self.set_key(&next_key);
        self.save(db)?;
        Ok(next_key)
    }
}

pub trait Separable<Host: Entity>: Entity + SetKey {
    fn get_optional_field(host: &mut Host) -> &mut Option<Self>;

    fn save_separate(key: &Self::Key, host: &mut Host, db: &Db) -> std::io::Result<()> {
        let opt_field = Self::get_optional_field(host);
        if opt_field.is_none() {
            return Err(std::io::Error::new(
                ErrorKind::NotFound,
                "No value was provided to separate from",
            ));
        }
        let mut separate_data = opt_field.take().unwrap();
        separate_data.set_key(key);
        separate_data.save(db)
    }

    fn restore(key: &Self::Key, host: &mut Host, db: &Db) -> std::io::Result<()> {
        *Self::get_optional_field(host) = Self::get(key, db)?;
        Ok(())
    }
}

pub trait SeparableIncrementable<Host: Entity>: Entity {
    fn save_next_separate(host: &mut Host, db: &Db) -> std::io::Result<Self::Key>;
}

impl<T, Host: Entity> SeparableIncrementable<Host> for T
where
    T: Entity + IncrementableEntity + Separable<Host>,
{
    fn save_next_separate(host: &mut Host, db: &Db) -> std::io::Result<Self::Key> {
        let opt_field = Self::get_optional_field(host);
        if opt_field.is_none() {
            return Err(std::io::Error::new(
                ErrorKind::NotFound,
                "No value was provided to separate from",
            ));
        }
        let mut separate_data = opt_field.take().unwrap();
        separate_data.save_next(db)
    }
}

pub trait AsBytes {
    fn as_bytes(&self) -> Vec<u8>;
}

impl AsBytes for String {
    fn as_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_owned()
    }
}

impl AsBytes for u32 {
    fn as_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl AsBytes for (u32, String) {
    fn as_bytes(&self) -> Vec<u8> {
        vec![self.0.to_be_bytes().to_vec(),self.1.as_bytes().to_vec()].concat()
    }
}

impl AsBytes for (u32, u32) {
    fn as_bytes(&self) -> Vec<u8> {
        vec![self.0.to_be_bytes().to_vec(),self.1.to_be_bytes().to_vec()].concat()
    }
}

impl AsBytes for Vec<u8> {
    fn as_bytes(&self) -> Vec<u8> {
        self.clone()
    }
}
