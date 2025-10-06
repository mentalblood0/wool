require "woollib/common"
require "woollib/Sweater"
require "woollib/exceptions"

require "./users/Users"

module Wool
  class Service
    mserializable

    getter sweater : Sweater
    getter users : Users

    def initialize(@sweater, @users)
    end

    enum Error
      UserIntegrationNotFound = 0
      OperationNotPermitted   = 1
    end

    def answer(s : Users::Site, pseudonym : String, c : Command(Sweater) | Command(Users))
      u = (@users.get s, pseudonym).not_nil! rescue return Error::UserIntegrationNotFound
      case c
      when Command(Sweater)
        return @users.push u.id, c
      when Command(Users)
        return Error::OperationNotPermitted unless u.role == User::Role::Moderator
        return c.exec @users
      end
    end
  end
end
