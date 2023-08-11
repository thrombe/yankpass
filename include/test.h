
#pragma once
#include "firebase/auth.h"
#include "firebase/auth/user.h"
#include "firebase/firestore.h"
#include "firebase/util.h"
#include <memory>

using firebase::App;
using firebase::firestore::Firestore;

class Store {
public:
  std::unique_ptr<App> app;
  std::shared_ptr<Firestore> db;

  Store();
  ~Store();

  void drop();
  void update_data(const char *);
};

std::unique_ptr<Store> create(const char *);
void myap(const std::string &);
